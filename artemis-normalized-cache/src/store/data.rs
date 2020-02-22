use flurry::{epoch, epoch::Guard};
use parking_lot::Mutex;
use std::{collections::HashMap, hash::Hash, sync::Arc};

type FlurryMap<K, V> = flurry::HashMap<K, V>;
type CacheMap<V> = Arc<FlurryMap<String, Mutex<HashMap<String, V>>>>;

type Records = CacheMap<serde_json::Value>;
type Links = CacheMap<Link>;
type QueryKeys = FlurryMap<u64, String>;
//type QueryKeysReverse = FlurryMap<String, u64>;

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum Link {
    Single(String),
    List(Vec<String>)
}

pub struct SerializedData {
    // Entities map
    records: HashMap<String, HashMap<String, serde_json::Value>>,
    // Connection keys
    links: HashMap<String, HashMap<String, Link>>
}

pub struct InMemoryData {
    records: Records,
    links: Links,
    query_keys: QueryKeys
    //entity_queries: QueryKeysReverse
}

impl InMemoryData {
    pub fn new() -> Self {
        Self {
            records: Records::default(),
            links: Links::default(),
            query_keys: QueryKeys::default()
            //entity_queries: QueryKeysReverse::default(),
        }
    }

    pub fn read_record<'g>(
        &'g self,
        entity_key: &String,
        field_key: &String,
        guard: &'g Guard
    ) -> Option<serde_json::Value> {
        self.records
            .get(entity_key, guard)
            .and_then(|entity| entity.lock().get(field_key).cloned())
    }

    pub fn read_link<'g>(
        &'g self,
        entity_key: &String,
        field_key: &String,
        guard: &'g Guard
    ) -> Option<Link> {
        self.links
            .get(entity_key, &guard)
            .and_then(|entity| entity.lock().get(field_key).cloned())
    }

    pub fn write_record(
        &self,
        entity_key: String,
        field_key: String,
        value: Option<serde_json::Value>
    ) {
        let guard = epoch::pin();
        let entity = self.records.get(&entity_key, &guard);

        if let Some(entity) = entity {
            let mut entity = entity.lock();
            if let Some(value) = value {
                entity.insert(field_key, value);
            } else {
                entity.remove(&field_key);
            }
        } else {
            if let Some(value) = value {
                let mut entity = HashMap::new();
                entity.insert(field_key, value);
                self.records.insert(entity_key, Mutex::new(entity), &guard);
            }
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
        let entity = self.links.get(&entity_key, &guard);

        if let Some(entity) = entity {
            let mut entity = entity.lock();
            entity.insert(field_key, link);
        } else {
            let mut entity_links = HashMap::new();
            entity_links.insert(field_key, link);
            self.links
                .insert(entity_key, Mutex::new(entity_links), &guard);
        }
    }

    /*    pub fn remove_link(&self, entity_key: &String, field_key: &String) -> Option<Link> {
        let guard = epoch::pin();
        self.links.get(entity_key, &guard).and_then(|entity_links| {
            let mut entity_links = entity_links.lock();
            let target_key = entity_links.get(field_key).cloned();
            if target_key.is_some() {
                entity_links.remove(field_key);
            }
            target_key
        })
    }*/

    pub fn get_entity_key_for_query<'g>(
        &'g self,
        query_key: &u64,
        guard: &'g Guard
    ) -> Option<&'g String> {
        self.query_keys.get(query_key, guard)
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

        self.records = Arc::new(records);
        self.links = Arc::new(links);
    }
}
