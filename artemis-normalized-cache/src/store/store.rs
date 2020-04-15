use crate::store::{
    data::{FieldKey, InMemoryData, Link, RefFieldKey},
    deserializer::ObjectDeserializer
};
use artemis::{
    codegen::FieldSelector,
    exchange::{Client, Operation, OperationOptions, OperationResult},
    utils::progressive_hash,
    GraphQLQuery, QueryError, RequestPolicy, Response
};
use flurry::{epoch, epoch::Guard};
use serde::de::Deserialize;
#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;
use std::{collections::HashMap, error::Error, fmt, sync::Arc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

pub struct Store {
    custom_keys: HashMap<&'static str, String>,
    data: InMemoryData
}

#[derive(Debug)]
pub enum StoreError {
    InvalidMetadata(String)
}
impl Error for StoreError {}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::InvalidMetadata(msg) => write!(f, "Invalid metadata: {}", msg)
        }
    }
}

const TYPENAME: FieldKey = FieldKey("__typename", String::new());

pub fn is_root(typename: &str) -> bool {
    typename == "Query" || typename == "Mutation" || typename == "Subscription"
}

/// A reference to the store used to run custom query updates
#[derive(Clone)]
pub struct QueryStore {
    pub(crate) store: Arc<Store>
}

impl QueryStore {
    /// Run a custom update function against the cache.
    ///
    /// # Parameters
    ///
    /// * `_query` - The [`GraphQLQuery`](../artemis/trait.GraphQLQuery.html) object for the query
    /// you want to update.
    /// * `variables` - The `Variables` for the query you want to update. It will only update
    /// cached results for that set of variables.
    /// * `updater_fn` - The custom updater function. This takes in an `Option<ResponseData>` that
    /// represents the current state and should return an `Option<ResponseData>` that represents
    /// the new state. `None` means deleting the entry in this context.
    /// The current state is cloned, so feel free to modify and return it.
    /// * `dependencies` - This is passed into the update closure and should simply be passed
    /// through.
    pub fn update_query<'a, Q: GraphQLQuery, F>(
        &'a self,
        _query: Q,
        variables: Q::Variables,
        updater_fn: F,
        dependencies: &mut Vec<String>
    ) where
        F: FnOnce(Option<Q::ResponseData>) -> Option<Q::ResponseData> + 'a
    {
        self.store
            .update_query::<Q, _>(variables, updater_fn, dependencies);
    }
}

impl From<Arc<Store>> for QueryStore {
    fn from(store: Arc<Store>) -> Self {
        Self { store }
    }
}

impl Store {
    pub fn update_query<'a, Q: GraphQLQuery, F>(
        &self,
        variables: Q::Variables,
        updater_fn: F,
        dependencies: &mut Vec<String>
    ) where
        F: FnOnce(Option<Q::ResponseData>) -> Option<Q::ResponseData> + 'a
    {
        let (query, meta) = Q::build_query(variables.clone());
        let key = progressive_hash(meta.query_key, &variables);
        let op = Operation {
            key,
            query,
            meta: meta.clone(),
            options: OperationOptions {
                extensions: None,
                extra_headers: None,
                request_policy: RequestPolicy::CacheOnly,
                url: "http://0.0.0.0".parse().unwrap()
            }
        };
        let data = self.read_query::<Q>(&op, dependencies);
        let updated_data = updater_fn(data);
        if let Some(updated_data) = updated_data {
            let result = OperationResult {
                key,
                meta,
                response: Response {
                    data: Some(updated_data),
                    errors: None,
                    debug_info: None
                }
            };
            self.write_query::<Q>(&result, &variables, false, dependencies)
                .unwrap();
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn update_query_js<Q: GraphQLQuery>(
        self: &Arc<Self>,
        variables: Value,
        updater_fn: js_sys::Function,
        dependencies: *mut usize
    ) where
        Q::Variables: DeserializeOwned
    {
        let variables: Q::Variables = serde_json::from_value(variables).unwrap();
        let updater = move |current_data: Option<Q::ResponseData>| -> Option<Q::ResponseData> {
            let this = JsValue::NULL;
            let current_data = serde_wasm_bindgen::to_value(&current_data).unwrap();
            let result = updater_fn.call1(&this, &current_data);
            serde_wasm_bindgen::from_value(result).unwrap()
        };
        let dependencies = dependencies as *mut _ as *mut HashSet<String>;
        let dependencies = unsafe { &mut *dependencies };
        self.update_query::<Q, _>(variables, updater, dependencies);
    }

    pub fn new(custom_keys: HashMap<&'static str, String>) -> Self {
        Self {
            data: InMemoryData::new(),
            custom_keys
        }
    }

    pub fn key_of_entity(
        &self,
        typename: &str,
        entity: &serde_json::Map<String, serde_json::Value>
    ) -> Option<String> {
        if is_root(typename) {
            return Some(typename.to_string());
        }

        let custom_id_key = self.custom_keys.get(typename);
        let id = if let Some(custom_key) = custom_id_key {
            entity.get(custom_key).and_then(|val| val.as_str())
        } else {
            entity
                .get("id")
                .or_else(|| entity.get("_id"))
                .and_then(|val| val.as_str())
        };

        id.map(|id| {
            let mut key = String::with_capacity(typename.len() + id.len() + 1);
            key.push_str(typename);
            key.push_str(":");
            key.push_str(id);
            key
        })
    }

    pub fn write_query<Q: GraphQLQuery>(
        &self,
        query: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        optimistic: bool,
        dependencies: &mut Vec<String>
    ) -> Result<(), QueryError> {
        if query.response.data.is_none() {
            return Ok(());
        }

        let data: Q::ResponseData = query.response.data.as_ref().unwrap().clone();
        let selection = Q::selection(variables);
        let data = serde_json::to_value(data)?;
        if !data.is_object() {
            return Ok(());
        }
        let data = data.as_object().unwrap();
        let key = query.meta.operation_type.to_string();

        let guard = epoch::pin();
        let optimistic_key = if optimistic { Some(query.key) } else { None };

        for field in selection {
            let field_name = self.get_selector_field_key(&field);
            let value = data.get(field_name).unwrap();
            self.store_data(
                optimistic_key,
                key.clone(),
                &field,
                value.clone(),
                dependencies,
                &guard
            )?;
        }

        if !optimistic {
            self.data.set_dependencies(query.key, dependencies.clone());
            self.data.collect_garbage();
        }

        Ok(())
    }

    pub fn clear_optimistic_layer(&self, query_key: u64) {
        self.data.clear_optimistic_layer(query_key);
    }

    fn store_object(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field: &FieldSelector,
        value: serde_json::Value,
        dependencies: &mut Vec<String>,
        guard: &Guard
    ) -> Result<(), StoreError> {
        let (field_name, args, typename, inner) = match field {
            FieldSelector::Object(name, args, typename, inner) => {
                (*name, args, typename.to_string(), inner.clone())
            }
            FieldSelector::Union(name, args, inner) => {
                let typename = self
                    .data
                    .read_record(&entity_key, (&TYPENAME).into(), guard)
                    .expect("Missing typename from union type. This is a codegen error.");
                let typename = typename
                    .as_str()
                    .expect("__typename has the wrong type. Should be string.");
                (*name, args, typename.to_string(), inner(typename))
            }
            _ => unreachable!()
        };

        let field_key = FieldKey(field_name, args.to_owned());
        if value.is_null() {
            self.data.write_link(entity_key, field_key, Link::Null);
            return Ok(());
        }
        let value = value.as_object().unwrap();
        let key = self.key_of_entity(&typename, value).ok_or_else(|| {
            StoreError::InvalidMetadata(format!(
                "Cache error: couldn't find index for {}:{}",
                entity_key, field_name
            ))
        })?;
        dependencies.push(key.clone());
        for field in &inner {
            let field_name = self.get_selector_field_key(&field);
            let value = value.get(field_name).unwrap();
            self.store_data(
                optimistic_key.clone(),
                key.clone(),
                field,
                value.clone(),
                dependencies,
                guard
            )?;
        }
        self.write_link(
            optimistic_key,
            entity_key,
            FieldKey(field_name, args.to_owned()),
            Some(Link::Single(key))
        );
        Ok(())
    }

    fn get_selector_field_key(&self, selector: &FieldSelector) -> &'static str {
        match selector {
            FieldSelector::Object(name, _, _, _) => *name,
            FieldSelector::Union(name, _, _) => *name,
            FieldSelector::Scalar(name, _) => *name
        }
    }

    fn store_array(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        selector: &FieldSelector,
        value: serde_json::Value,
        dependencies: &mut Vec<String>,
        guard: &Guard
    ) -> Result<(), StoreError> {
        let (field_name, args, typename, inner) = match selector {
            FieldSelector::Scalar(field_name, args) => {
                let field_key = FieldKey(*field_name, args.to_owned());
                self.write_record(optimistic_key, entity_key, field_key, Some(value));
                return Ok(());
            }
            FieldSelector::Object(field_name, args, typename, inner) => {
                (*field_name, args, typename.to_string(), inner.clone())
            }
            FieldSelector::Union(field_name, args, inner) => {
                let typename = self
                    .data
                    .read_record(&entity_key, (&TYPENAME).into(), guard)
                    .expect("Missing typename from union type. This is a codegen error.");
                let typename = typename
                    .as_str()
                    .expect("__typename has the wrong type. Should be string.");
                (*field_name, args, typename.to_string(), inner(typename))
            }
        };

        let field_key = FieldKey(field_name, args.to_owned());

        if value.is_null() {
            self.write_link(optimistic_key, entity_key, field_key, Some(Link::Null));
            return Ok(());
        }

        let values = value.as_array().unwrap();
        let mut keys = Vec::new();
        for value in values {
            let value = value.as_object().unwrap();
            let key = self.key_of_entity(&typename, value).ok_or_else(|| {
                StoreError::InvalidMetadata(format!(
                    "Cache error: couldn't find index for {}:{}",
                    entity_key, field_key
                ))
            })?;
            dependencies.push(key.clone());

            for selector in inner.iter() {
                let field_name = self.get_selector_field_key(selector);
                let value = value.get(field_name).unwrap();
                self.store_data(
                    optimistic_key.clone(),
                    key.clone(),
                    selector,
                    value.clone(),
                    dependencies,
                    guard
                )?;
            }

            keys.push(key);
        }

        self.write_link(
            optimistic_key,
            entity_key,
            field_key,
            Some(Link::List(keys))
        );
        Ok(())
    }

    fn store_data(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        selector: &FieldSelector,
        data: serde_json::Value,
        dependencies: &mut Vec<String>,
        guard: &Guard
    ) -> Result<(), StoreError> {
        match selector {
            FieldSelector::Object(_, _, _, _) | FieldSelector::Union(_, _, _) => {
                if data.is_array() {
                    self.store_array(
                        optimistic_key,
                        entity_key,
                        selector,
                        data,
                        dependencies,
                        guard
                    )?;
                } else {
                    self.store_object(
                        optimistic_key,
                        entity_key,
                        selector,
                        data,
                        dependencies,
                        guard
                    )?;
                }
            }
            FieldSelector::Scalar(field_name, args) => {
                self.write_record(
                    optimistic_key,
                    entity_key,
                    FieldKey(*field_name, args.to_owned()),
                    Some(data)
                );
            }
        }
        Ok(())
    }

    fn write_record(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field_key: FieldKey,
        value: Option<serde_json::Value>
    ) {
        if let Some(optimistic_key) = optimistic_key {
            self.data
                .write_record_optimistic(optimistic_key, entity_key, field_key, value);
        } else {
            self.data.write_record(entity_key, field_key, value);
        }
    }

    fn write_link(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field_key: FieldKey,
        value: Option<Link>
    ) {
        if let Some(optimistic_key) = optimistic_key {
            self.data
                .write_link_optimistic(optimistic_key, entity_key, field_key, value);
        } else if let Some(value) = value {
            // Non-optimistic writes only support insertion
            self.data.write_link(entity_key, field_key, value);
        }
    }

    pub fn read_query<Q: GraphQLQuery>(
        &self,
        query: &Operation<Q::Variables>,
        dependencies: *mut Vec<String>
    ) -> Option<Q::ResponseData> {
        let root_key = query.meta.operation_type.to_string();
        let selection = Q::selection(&query.query.variables);
        let guard = epoch::pin();
        let deserializer =
            ObjectDeserializer::new(&self.data, &selection, &root_key, &guard, dependencies);
        /*        let value = self.read_entity(root_key, &selection, dependencies)?;
        let data: Q::ResponseData =
            serde_json::from_value(value).expect("Cache result didn't match type");*/
        let data = Q::ResponseData::deserialize(deserializer);
        match data {
            Ok(data) => Some(data),
            Err(e) if e.is_missing() => None,
            Err(e) => panic!("{}", e)
        }
    }

    #[inline]
    fn field_key<'a>(field_name: &'static str, args: &'a String) -> RefFieldKey<'a> {
        RefFieldKey(field_name, args)
    }

    fn invalidate_union(
        &self,
        optimistic_key: Option<u64>,
        entity_key: &str,
        subselection: &dyn Fn(&str) -> Vec<FieldSelector>,
        invalidated: &mut Vec<String>,
        guard: &Guard
    ) {
        let typename = self
            .data
            .read_record(entity_key, (&TYPENAME).into(), guard)
            .expect("Missing typename from union type. This is a codegen error.");
        let typename = typename.as_str().unwrap();
        let subselection = subselection(typename);
        self.invalidate_selection(
            optimistic_key,
            entity_key,
            &subselection,
            invalidated,
            guard
        );
    }

    pub fn invalidate_query<Q: GraphQLQuery>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        optimistic: bool,
        dependencies: &mut Vec<String>
    ) {
        if result.response.data.is_none() {
            return;
        }
        let key = result.meta.operation_type.to_string();

        let selection = Q::selection(variables);
        let guard = epoch::pin();
        let optimistic_key = if optimistic { Some(result.key) } else { None };

        self.invalidate_selection(optimistic_key, &key, &selection, dependencies, &guard);

        if !optimistic {
            self.data.clear_optimistic_layer(result.key);
        }
    }

    pub fn rerun_queries<C: Client>(
        &self,
        mut entities: Vec<String>,
        originating_query: u64,
        client: &C
    ) {
        entities.sort_unstable();
        entities.dedup();
        //println!("Rerun dependencies: {:?}", entities);

        let mut queries: Vec<_> = entities
            .iter()
            .filter(|it| *it != "Query")
            .flat_map(|entity| self.data.get_dependencies(entity))
            .filter(|it| *it != originating_query)
            .collect();

        queries.sort_unstable();
        queries.dedup();

        for query in queries {
            client.rerun_query(query);
        }
    }

    fn invalidate_selection(
        &self,
        optimistic_key: Option<u64>,
        entity_key: &str,
        selection: &[FieldSelector],
        invalidated: &mut Vec<String>,
        guard: &Guard
    ) {
        if entity_key != "Mutation" {
            invalidated.push(entity_key.to_string());
        }
        for field in selection {
            match field {
                FieldSelector::Scalar(field_name, args) => {
                    self.write_record(
                        optimistic_key,
                        entity_key.to_string(),
                        FieldKey(*field_name, args.to_owned()),
                        None
                    );
                }
                FieldSelector::Object(field_name, args, _, subselection) => {
                    let field_key = Self::field_key(*field_name, args);
                    if let Some(link) = self.data.read_link(entity_key, field_key, &guard) {
                        match link {
                            Link::Single(ref entity_key) => self.invalidate_selection(
                                optimistic_key,
                                entity_key,
                                subselection,
                                invalidated,
                                guard
                            ),
                            Link::List(ref entity_keys) => {
                                for entity_key in entity_keys {
                                    self.invalidate_selection(
                                        optimistic_key.clone(),
                                        entity_key,
                                        subselection,
                                        invalidated,
                                        guard
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                FieldSelector::Union(field_name, args, subselection) => {
                    let field_key = Self::field_key(*field_name, args);
                    if let Some(link) = self.data.read_link(entity_key, field_key, &guard) {
                        match link {
                            Link::Single(ref entity_key) => self.invalidate_union(
                                optimistic_key,
                                entity_key,
                                &**subselection,
                                invalidated,
                                guard
                            ),
                            Link::List(ref entity_keys) => {
                                for entity_key in entity_keys {
                                    self.invalidate_union(
                                        optimistic_key.clone(),
                                        entity_key,
                                        &**subselection,
                                        invalidated,
                                        guard
                                    )
                                }
                            }
                            Link::Null => {}
                        }
                    }
                }
            }
        }
    }
}
