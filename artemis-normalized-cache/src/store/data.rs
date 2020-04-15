use crossbeam_epoch::Atomic;
use flurry::{epoch, epoch::Guard};
use fnv::FnvBuildHasher;
use parking_lot::Mutex;
use serde_json::Value;
use std::{
    collections::HashSet,
    fmt,
    hash::Hash,
    mem::ManuallyDrop,
    ptr,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc
    }
};

#[derive(Hash, Eq, PartialEq)]
pub struct FieldKey(pub &'static str, pub String);
pub struct RefFieldKey<'a>(pub &'static str, pub &'a String);

impl<'a> From<&'a FieldKey> for RefFieldKey<'a> {
    fn from(key: &'a FieldKey) -> Self {
        RefFieldKey(key.0, &key.1)
    }
}

fn deref_field_key(key: &RefFieldKey<'_>) -> ManuallyDrop<FieldKey> {
    // The 24-byte string headers of `a` and `b` may not be adjacent in
    // memory. Copy them (just the headers) so that they are adjacent. This
    // makes a `(String, String)` backed by the same data as `a` and `b`.
    let k = unsafe { FieldKey(key.0, ptr::read(key.1)) };

    // Make sure not to drop the strings, even if `get` panics. The caller
    // or whoever owns `a` and `b` will drop them.
    ManuallyDrop::new(k)
}

impl fmt::Display for FieldKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.0, self.1)
    }
}

type Hasher = FnvBuildHasher;

type HashMap<K, V> = std::collections::HashMap<K, V, Hasher>;
type FlurryMap<K, V> = flurry::HashMap<K, V, Hasher>;
type CacheMap<V> = Arc<FlurryMap<String, Mutex<HashMap<FieldKey, V>>>>;

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

type Records = OptimisticMap<Atomic<Value>>;
type Links = OptimisticMap<Atomic<Link>>;
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
    records: HashMap<String, HashMap<FieldKey, Value>>,
    // Connection keys
    links: HashMap<String, HashMap<FieldKey, Link>>
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

    pub fn set_dependencies(&self, query_key: u64, mut dependencies: Vec<String>) {
        let mut dependents = self.dependencies.lock();
        dependencies.sort_unstable();
        dependencies.dedup();
        for dependency in dependencies {
            if &dependency == "Query" {
                continue;
            }
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

    pub fn get_dependencies(&self, entity_key: &str) -> Vec<u64> {
        let dependencies = self.dependencies.lock();
        dependencies
            .get(entity_key)
            .map(|entity| entity.iter().cloned().collect())
            .unwrap_or_else(Vec::new)
    }

    /// Return reference
    pub fn read_record<'g>(
        &'g self,
        entity_key: &str,
        field_key: RefFieldKey,
        guard: &'g Guard
    ) -> Option<&'g Value> {
        self.records
            .optimistic
            .values(guard)
            .find_map(|layer| {
                layer
                    // Need to get the field itself to check if it exists on this layer
                    // If it exists but is none, this will cause the function to return None to reflect deletions
                    .get(entity_key, guard)
                    .and_then(|entity| {
                        entity
                            .lock()
                            .get(&deref_field_key(&field_key))
                            .map(|record| record.as_ref().map(|val| load_value(&val, guard)))
                    })
            })
            .or_else(|| {
                self.records
                    .base
                    .get(entity_key, guard)
                    .and_then(|entity| {
                        entity
                            .lock()
                            .get(&deref_field_key(&field_key))
                            .map(|val| load_value(val, guard))
                    })
                    .map(Option::Some)
            })
            .and_then(|res| res)
    }

    pub fn read_link<'g>(
        &'g self,
        entity_key: &str,
        field_key: RefFieldKey,
        guard: &'g Guard
    ) -> Option<&'g Link> {
        self.links
            .optimistic
            .values(guard)
            .find_map(|layer| {
                layer.get(entity_key, guard).and_then(|entity| {
                    entity
                        .lock()
                        .get(&deref_field_key(&field_key))
                        .map(|field| field.as_ref().map(|val| load_link(val, guard)))
                })
            })
            .or_else(|| {
                self.links
                    .base
                    .get(entity_key, guard)
                    .and_then(|entity| {
                        entity
                            .lock()
                            .get(&deref_field_key(&field_key))
                            .map(|val| load_link(val, guard))
                    })
                    .map(Option::Some)
            })
            .and_then(|res| res)
    }

    pub fn write_record(
        &self,
        entity_key: String,
        field_key: FieldKey,
        value: Option<serde_json::Value>
    ) {
        let guard = epoch::pin();
        let entity = self.records.base.get(&entity_key, &guard);

        if let Some(entity) = entity {
            let mut entity = entity.lock();
            if let Some(value) = value {
                entity.insert(field_key, Atomic::new(value));
            } else {
                entity.remove(&field_key);
                self.remove_link(&entity_key, (&field_key).into());
            }
        } else if let Some(value) = value {
            let mut entity = HashMap::default();
            entity.insert(field_key, Atomic::new(value));
            self.records
                .base
                .insert(entity_key, Mutex::new(entity), &guard);
        }
    }

    pub fn clear_optimistic_layer(&self, optimistic_key: u64) {
        let guard = epoch::pin();
        self.records.optimistic.remove(&optimistic_key, &guard);
        self.links.optimistic.remove(&optimistic_key, &guard);
    }

    pub fn write_record_optimistic(
        &self,
        optimistic_key: u64,
        entity_key: String,
        field_key: FieldKey,
        value: Option<serde_json::Value>
    ) {
        let guard = epoch::pin();
        let layer = self.records.optimistic.get(&optimistic_key, &guard);
        if let Some(layer) = layer {
            if let Some(entity) = layer.get(&entity_key, &guard) {
                let mut entity = entity.lock();
                entity.insert(field_key, value.map(Atomic::new));
            } else {
                let mut entity = HashMap::default();
                entity.insert(field_key, value.map(Atomic::new));
                layer.insert(entity_key, Mutex::new(entity), &guard);
            }
        } else {
            let layer = CacheMap::default();
            let mut entity = HashMap::default();
            entity.insert(field_key, value.map(Atomic::new));
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

    pub fn write_link(&self, entity_key: String, field_key: FieldKey, link: Link) {
        let guard = epoch::pin();
        let entity = self.links.base.get(&entity_key, &guard);

        if let Some(entity) = entity {
            let mut entity = entity.lock();
            self.update_link_ref_count(
                entity.get(&field_key).map(|link| load_link(link, &guard)),
                -1,
                &guard
            );
            self.update_link_ref_count(Some(&link), 1, &guard);
            entity.insert(field_key, Atomic::new(link));
        } else {
            let mut entity_links = HashMap::default();
            self.update_link_ref_count(Some(&link), 1, &guard);
            entity_links.insert(field_key, Atomic::new(link));
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
        entity_key: &str,
        field_key: &FieldKey,
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
            let existing = self.read_link(entity_key, field_key.into(), guard);
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
        field_key: FieldKey,
        link: Option<Link>
    ) {
        let guard = epoch::pin();
        let layer = self.links.optimistic.get(&optimistic_key, &guard);
        if let Some(layer) = layer {
            if let Some(entity) = layer.get(&entity_key, &guard) {
                let mut entity = entity.lock();
                if let Some(field) = entity.get(&field_key) {
                    let field = field.as_ref().map(|field| load_link(field, &guard));
                    self.update_link_ref_count_optimistic(
                        &entity_key,
                        &field_key,
                        field,
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
                entity.insert(field_key, link.map(Atomic::new));
            } else {
                let mut entity = HashMap::default();
                self.update_link_ref_count_optimistic(
                    &entity_key,
                    &field_key,
                    link.as_ref(),
                    1,
                    &guard
                );
                entity.insert(field_key, link.map(Atomic::new));
                layer.insert(entity_key, Mutex::new(entity), &guard);
            }
        } else {
            let layer = FlurryMap::default();
            let mut entity = HashMap::default();
            self.update_link_ref_count_optimistic(
                &entity_key,
                &field_key,
                link.as_ref(),
                1,
                &guard
            );
            entity.insert(field_key, link.map(Atomic::new));
            layer.insert(entity_key, Mutex::new(entity), &guard);
            self.links
                .optimistic
                .insert(optimistic_key, Arc::new(layer), &guard);
        }
    }

    pub fn remove_link(&self, entity_key: &str, field_key: RefFieldKey) {
        let guard = epoch::pin();
        if let Some(entity_links) = self.links.base.get(entity_key, &guard) {
            let mut entity_links = entity_links.lock();
            if entity_links.remove(&deref_field_key(&field_key)).is_some() {
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

    pub fn update_ref_count(&self, key: &str, by: isize, guard: &Guard) {
        if let Some(ref_count) = self.ref_counts.get(key, guard) {
            let new_val = ref_count.fetch_add(by, Ordering::SeqCst) + by;
            if new_val <= 0 {
                self.gc_queue.insert(key.to_string(), (), guard);
            } else {
                self.gc_queue.remove(key, guard);
            }
        } else if by >= 0 {
            self.ref_counts
                .insert(key.to_string(), AtomicIsize::new(by), guard);
        } else {
            panic!("Tried to decrease ref count of non-existing entity {}. This is an error with the cache code.", key);
        }
    }

    #[allow(unused)]
    pub fn hydrate_data(&mut self, state: SerializedData) {
        let guard = epoch::pin();

        let records = FlurryMap::default();
        for (key, value) in state.records {
            let value = value
                .into_iter()
                .map(|(k, v)| (k, Atomic::new(v)))
                .collect();
            records.insert(key, Mutex::new(value), &guard);
        }

        let links = FlurryMap::default();
        for (key, value) in state.links {
            let value = value
                .into_iter()
                .map(|(k, v)| (k, Atomic::new(v)))
                .collect();
            links.insert(key, Mutex::new(value), &guard);
        }

        let links = Links {
            base: Arc::new(links),
            optimistic: FlurryMap::default()
        };
        let records = Records {
            base: Arc::new(records),
            optimistic: FlurryMap::default()
        };

        self.records = records;
        self.links = links;
    }
}

fn load_link<'g>(link: &Atomic<Link>, guard: &'g Guard) -> &'g Link {
    let val = link.load(Ordering::SeqCst, guard);

    assert!(!val.is_null());

    // safety: the lifetime of the reference is bound to the guard
    // supplied which means that the memory will not be modified
    // until at least after the guard goes out of scope
    unsafe { val.as_ref() }.unwrap()
}

fn load_value<'g>(value: &Atomic<Value>, guard: &'g Guard) -> &'g Value {
    let val = value.load(Ordering::SeqCst, guard);

    assert!(!val.is_null());

    // safety: the lifetime of the reference is bound to the guard
    // supplied which means that the memory will not be modified
    // until at least after the guard goes out of scope
    unsafe { val.as_ref() }.unwrap()
}
