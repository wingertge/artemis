use flurry::{epoch, epoch::Guard};
use parking_lot::Mutex;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc
    }
};

type FlurryMap<K, V> = flurry::HashMap<K, V>;
type CacheMap<V> = Arc<FlurryMap<String, Mutex<HashMap<String, V>>>>;
struct OptimisticMap<V: 'static + Send> {
    base: CacheMap<V>,
    optimistic: FlurryMap<u64, CacheMap<Option<V>>>
}

impl<V: 'static + Send> Default for OptimisticMap<V> {
    fn default() -> Self {
        Self {
            base: CacheMap::default(),
            optimistic: FlurryMap::default()
        }
    }
}

type Records = OptimisticMap<serde_json::Value>;
type Links = OptimisticMap<Link>;
type Dependents = Arc<Mutex<HashMap<String, HashSet<u64>>>>;
type RefCounts = FlurryMap<String, AtomicIsize>;

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum Link {
    Single(String),
    List(Vec<String>),
    Null
}

pub struct SerializedData {
    // Entities map
    records: HashMap<String, HashMap<String, serde_json::Value>>,
    // Connection keys
    links: HashMap<String, HashMap<String, Link>>
}

#[derive(Default)]
pub struct InMemoryData {
    records: Records,
    links: Links,
    dependencies: Dependents,
    ref_counts: RefCounts,
    gc_queue: FlurryMap<String, ()>
}

impl InMemoryData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_dependencies(&self, query_key: u64, dependencies: HashSet<String>) {
        let mut dependents = self.dependencies.lock();
        for dependency in dependencies {
            let depending_queries = dependents.get_mut(&dependency);
            if let Some(dependency_set) = depending_queries {
                dependency_set.insert(query_key.clone());
            } else {
                let mut deps = HashSet::new();
                deps.insert(query_key);

                dependents.insert(dependency, deps);
            }
        }
    }

    pub fn get_dependencies(&self, entity_key: &String) -> Vec<u64> {
        let dependencies = self.dependencies.lock();
        dependencies
            .get(entity_key)
            .map(|entity| entity.iter().cloned().collect())
            .unwrap_or_else(|| Vec::new())
    }

    pub fn read_record<'g>(
        &'g self,
        entity_key: &String,
        field_key: &String,
        guard: &'g Guard
    ) -> Option<serde_json::Value> {
        self.records
            .optimistic
            .values(guard)
            .find_map(|layer| {
                layer
                    // Need to get the field itself to check if it exists on this layer
                    // If it exists but is none, this will cause the function to return None to reflect deletions
                    .get(entity_key, guard)
                    .and_then(|entity| entity.lock().get(field_key).cloned())
            })
            .or_else(|| {
                self.records
                    .base
                    .get(entity_key, guard)
                    .and_then(|entity| entity.lock().get(field_key).cloned())
                    .map(|res| Some(res))
            })
            .and_then(|res| res)
    }

    pub fn read_link<'g>(
        &'g self,
        entity_key: &String,
        field_key: &String,
        guard: &'g Guard
    ) -> Option<Link> {
        self.links
            .optimistic
            .values(guard)
            .find_map(|layer| {
                layer
                    .get(entity_key, guard)
                    .and_then(|entity| entity.lock().get(field_key).cloned())
            })
            .or_else(|| {
                self.links
                    .base
                    .get(entity_key, guard)
                    .and_then(|entity| entity.lock().get(field_key).cloned())
                    .map(|res| Some(res))
            })
            .and_then(|res| res)
    }

    pub fn write_record(
        &self,
        entity_key: String,
        field_key: String,
        value: Option<serde_json::Value>
    ) {
        let guard = epoch::pin();
        let entity = self.records.base.get(&entity_key, &guard);

        if let Some(entity) = entity {
            let mut entity = entity.lock();
            if let Some(value) = value {
                entity.insert(field_key, value);
            } else {
                entity.remove(&field_key);
                self.remove_link(&entity_key, &field_key);
            }
        } else {
            if let Some(value) = value {
                let mut entity = HashMap::new();
                entity.insert(field_key, value);
                self.records
                    .base
                    .insert(entity_key, Mutex::new(entity), &guard);
            }
        }
    }

    pub fn clear_optimistic_layer(&self, optimistic_key: &u64) {
        let guard = epoch::pin();
        self.records.optimistic.remove(optimistic_key, &guard);
        self.links.optimistic.remove(optimistic_key, &guard);
    }

    pub fn write_record_optimistic(
        &self,
        optimistic_key: u64,
        entity_key: String,
        field_key: String,
        value: Option<serde_json::Value>
    ) {
        let guard = epoch::pin();
        let layer = self.records.optimistic.get(&optimistic_key, &guard);
        if let Some(layer) = layer {
            if let Some(entity) = layer.get(&entity_key, &guard) {
                let mut entity = entity.lock();
                entity.insert(field_key, value);
            } else {
                let mut entity = HashMap::new();
                entity.insert(field_key, value);
                layer.insert(entity_key, Mutex::new(entity), &guard);
            }
        } else {
            let layer = CacheMap::default();
            let mut entity = HashMap::new();
            entity.insert(field_key, value);
            layer.insert(entity_key, Mutex::new(entity), &guard);
            self.records
                .optimistic
                .insert(optimistic_key, layer, &guard);
        }
    }

    /*
    pub fn has_field(&self, entity_key: &String, field_key: &String) -> bool {
        let guard = epoch::pin();
        self.read_record(entity_key, field_key, &guard).is_some()
            || self.read_link(entity_key, field_key, &guard).is_some()
    }
    */

    pub fn write_link(&self, entity_key: String, field_key: String, link: Link) {
        let guard = epoch::pin();
        let entity = self.links.base.get(&entity_key, &guard);

        if let Some(entity) = entity {
            let mut entity = entity.lock();
            self.update_link_ref_count(entity.get(&field_key), -1, &guard);
            self.update_link_ref_count(Some(&link), 1, &guard);
            entity.insert(field_key, link);
        } else {
            let mut entity_links = HashMap::new();
            self.update_link_ref_count(Some(&link), 1, &guard);
            entity_links.insert(field_key, link);
            self.links
                .base
                .insert(entity_key, Mutex::new(entity_links), &guard);
        }
    }

    fn update_link_ref_count<'g>(&self, link: Option<&'g Link>, by: isize, guard: &'g Guard) {
        match link {
            Some(Link::Single(entity)) => self.update_ref_count(entity, by, guard),
            Some(Link::List(entities)) => {
                for entity in entities {
                    self.update_ref_count(entity, by, guard);
                }
            }
            _ => {}
        }
    }

    fn update_link_ref_count_optimistic<'g>(
        &self,
        entity_key: &String,
        field_key: &String,
        link: Option<&'g Link>,
        by: isize,
        guard: &'g Guard
    ) {
        if let Some(link) = link {
            match link {
                Link::Single(entity) => self.update_ref_count(entity, by, guard),
                Link::List(entities) => {
                    for entity in entities {
                        self.update_ref_count(entity, by, guard);
                    }
                }
                _ => {}
            }
        } else {
            let existing = self.read_link(entity_key, field_key, guard);
            match existing {
                Some(Link::Single(entity)) => self.update_ref_count(&entity, -by, guard),
                Some(Link::List(entities)) => {
                    for entity in entities {
                        self.update_ref_count(&entity, -by, guard);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn write_link_optimistic(
        &self,
        optimistic_key: u64,
        entity_key: String,
        field_key: String,
        link: Option<Link>
    ) {
        let guard = epoch::pin();
        let layer = self.links.optimistic.get(&optimistic_key, &guard);
        if let Some(layer) = layer {
            if let Some(entity) = layer.get(&entity_key, &guard) {
                let mut entity = entity.lock();
                if let Some(field) = entity.get(&field_key) {
                    self.update_link_ref_count_optimistic(
                        &entity_key,
                        &field_key,
                        field.as_ref(),
                        -1,
                        &guard
                    );
                }
                self.update_link_ref_count_optimistic(
                    &entity_key,
                    &field_key,
                    link.as_ref(),
                    1,
                    &guard
                );
                entity.insert(field_key, link);
            } else {
                let mut entity = HashMap::new();
                self.update_link_ref_count_optimistic(
                    &entity_key,
                    &field_key,
                    link.as_ref(),
                    1,
                    &guard
                );
                entity.insert(field_key, link);
                layer.insert(entity_key, Mutex::new(entity), &guard);
            }
        } else {
            let layer = FlurryMap::new();
            let mut entity = HashMap::new();
            self.update_link_ref_count_optimistic(
                &entity_key,
                &field_key,
                link.as_ref(),
                1,
                &guard
            );
            entity.insert(field_key, link);
            layer.insert(entity_key, Mutex::new(entity), &guard);
            self.links
                .optimistic
                .insert(optimistic_key, Arc::new(layer), &guard);
        }
    }

    pub fn remove_link(&self, entity_key: &String, field_key: &String) {
        let guard = epoch::pin();
        if let Some(entity_links) = self.links.base.get(entity_key, &guard) {
            let mut entity_links = entity_links.lock();
            if entity_links.remove(field_key).is_some() {
                self.update_ref_count(entity_key, -1, &guard);
            }
        }
    }

    pub fn collect_garbage(&self) {
        let guard = epoch::pin();
        for key in self.gc_queue.keys(&guard) {
            self.records.base.remove(key, &guard);
            self.gc_queue.remove(key, &guard);
        }
    }

    pub fn update_ref_count(&self, key: &String, by: isize, guard: &Guard) {
        if let Some(ref_count) = self.ref_counts.get(key, guard) {
            let new_val = ref_count.fetch_add(by, Ordering::SeqCst) + by;
            if new_val <= 0 {
                self.gc_queue.insert(key.clone(), (), guard);
            } else {
                self.gc_queue.remove(key, guard);
            }
        } else if by >= 0 {
            self.ref_counts
                .insert(key.clone(), AtomicIsize::new(by), guard);
        } else {
            panic!("Tried to decrease ref count of non-existing entity {}. This is an error with the cache code.", key);
        }
    }

    #[allow(unused)]
    pub fn hydrate_data(&mut self, state: SerializedData) {
        let guard = epoch::pin();

        let records = FlurryMap::new();
        for (key, value) in state.records {
            records.insert(key, Mutex::new(value), &guard);
        }

        let links = FlurryMap::new();
        for (key, value) in state.links {
            links.insert(key, Mutex::new(value), &guard);
        }

        let links = Links {
            base: Arc::new(links),
            optimistic: FlurryMap::new()
        };
        let records = Records {
            base: Arc::new(records),
            optimistic: FlurryMap::new()
        };

        self.records = records;
        self.links = links;
    }
}
