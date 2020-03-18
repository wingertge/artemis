use crate::store::data::{InMemoryData, Link};
use artemis::{
    exchanges::Client, types::OperationOptions, FieldSelector, GraphQLQuery, Operation,
    OperationResult, QueryError, RequestPolicy, Response
};
use flurry::{epoch, epoch::Guard};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    sync::Arc
};
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

lazy_static! {
    static ref TYPENAME: String = String::from("__typename");
}

pub fn is_root(typename: &str) -> bool {
    typename == "Query" || typename == "Mutation" || typename == "Subscription"
}

#[derive(Clone)]
pub struct QueryStore {
    pub(crate) store: Arc<Store>
}

impl QueryStore {
    pub fn update_query<Q: GraphQLQuery, F>(
        &self,
        _query: Q,
        variables: Q::Variables,
        updater_fn: F,
        dependencies: &mut HashSet<String>
    ) where
        F: Fn(Option<Q::ResponseData>) -> Option<Q::ResponseData>
    {
        self.store.update_query::<Q, _>(variables, updater_fn, dependencies);
    }
}

impl From<Arc<Store>> for QueryStore {
    fn from(store: Arc<Store>) -> Self {
        Self { store }
    }
}

impl Store {
    pub fn update_query<Q: GraphQLQuery, F>(
        &self,
        variables: Q::Variables,
        updater_fn: F,
        dependencies: &mut HashSet<String>
    ) where
        F: Fn(Option<Q::ResponseData>) -> Option<Q::ResponseData>
    {
        let (query, meta) = Q::build_query(variables.clone());
        let op = Operation {
            query,
            meta: meta.clone(),
            options: OperationOptions {
                extensions: None,
                extra_headers: None,
                request_policy: RequestPolicy::CacheOnly,
                url: "http://0.0.0.0".parse().unwrap()
            }
        };
        let data = self.store.read_query::<Q>(&op, dependencies);
        let updated_data = updater_fn(data);
        if let Some(updated_data) = updated_data {
            let result = OperationResult {
                meta,
                response: Response {
                    data: Some(updated_data),
                    errors: None,
                    debug_info: None
                }
            };
            self.store
                .write_query::<Q>(&result, &variables, false, dependencies)
                .unwrap();
        }
    }

    pub fn update_query_js<Q: GraphQLQuery>(
        self: &Arc<Self>,
        variables: Value,
        updater_fn: js_sys::Function,
        dependencies: *mut usize
    ) {
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

        id.map(|id| format!("{}:{}", typename, id))
    }

    pub fn write_query<Q: GraphQLQuery>(
        &self,
        query: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        optimistic: bool,
        dependencies: &mut HashSet<String>
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
        let optimistic_key = if optimistic {
            Some(query.meta.query_key.clone())
        } else {
            None
        };

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
            self.data
                .set_dependencies(query.meta.query_key.clone(), dependencies.clone());
            self.data.collect_garbage();
        }

        Ok(())
    }

    pub fn clear_optimistic_layer(&self, query_key: &u64) {
        self.data.clear_optimistic_layer(query_key);
    }

    fn store_object(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field: &FieldSelector,
        value: serde_json::Value,
        dependencies: &mut HashSet<String>,
        guard: &Guard
    ) -> Result<(), StoreError> {
        let (field_name, args, inner) = match field {
            FieldSelector::Object(name, args, _, inner) => (name, args, inner.clone()),
            FieldSelector::Union(name, args, _, inner) => {
                let typename = self
                    .data
                    .read_record(&entity_key, &TYPENAME, guard)
                    .expect("Missing typename from union type. This is a codegen error.");
                let typename = typename
                    .as_str()
                    .expect("__typename has the wrong type. Should be string.");
                (name, args, inner(typename))
            }
            _ => unreachable!()
        };

        let field_key = format!("{}{}", field_name, args);
        if value.is_null() {
            self.data.write_link(entity_key, field_key, Link::Null);
            return Ok(());
        }
        let value = value.as_object().unwrap();
        let key = self.key_of_entity(&"TODO", value).ok_or_else(|| {
            StoreError::InvalidMetadata(format!(
                "Cache error: couldn't find index for {}:{}",
                entity_key, field_name
            ))
        })?;
        dependencies.insert(key.clone());
        for ref field in inner {
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
            format!("{}{}", field_name, args),
            Some(Link::Single(key))
        );
        Ok(())
    }

    fn get_selector_field_key<'a, 's>(&'a self, selector: &'s FieldSelector) -> &'s String {
        match selector {
            FieldSelector::Object(name, _, _, _) => name,
            FieldSelector::Union(name, _, _, _) => name,
            FieldSelector::Scalar(name, _) => name
        }
    }

    fn store_array(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        selector: &FieldSelector,
        value: serde_json::Value,
        dependencies: &mut HashSet<String>,
        guard: &Guard
    ) -> Result<(), StoreError> {
        let (field_name, args, inner) = match selector {
            FieldSelector::Scalar(field_name, args) => {
                let field_key = format!("{}{}", field_name, args);
                self.write_record(optimistic_key, entity_key, field_key, Some(value));
                return Ok(());
            }
            FieldSelector::Object(field_name, args, _, inner) => (field_name, args, inner.clone()),
            FieldSelector::Union(field_name, args, _, inner) => {
                let typename = self
                    .data
                    .read_record(&entity_key, &TYPENAME, guard)
                    .expect("Missing typename from union type. This is a codegen error.");
                let typename = typename
                    .as_str()
                    .expect("__typename has the wrong type. Should be string.");
                (field_name, args, inner(typename))
            }
        };

        let field_key = format!("{}{}", field_name, args);

        if value.is_null() {
            self.write_link(optimistic_key, entity_key, field_key, Some(Link::Null));
            return Ok(());
        }

        let values = value.as_array().unwrap();
        let mut keys = Vec::new();
        for value in values {
            let value = value.as_object().unwrap();
            let key = self.key_of_entity(&"TODO", value).ok_or_else(|| {
                StoreError::InvalidMetadata(format!(
                    "Cache error: couldn't find index for {}:{}",
                    entity_key, field_key
                ))
            })?;
            dependencies.insert(key.clone());

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
        dependencies: &mut HashSet<String>,
        guard: &Guard
    ) -> Result<(), StoreError> {
        match selector {
            FieldSelector::Object(_, _, _, _) | FieldSelector::Union(_, _, _, _) => {
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
                let field_key = format!("{}{}", field_name, args);
                self.write_record(optimistic_key, entity_key, field_key, Some(data));
            }
        }
        Ok(())
    }

    fn write_record(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field_key: String,
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
        field_key: String,
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
        dependencies: &mut HashSet<String>
    ) -> Option<Q::ResponseData> {
        let root_key = query.meta.operation_type.to_string();
        let selection = Q::selection(&query.query.variables);
        let value = self.read_entity(&root_key, &selection, dependencies)?;
        let data: Q::ResponseData =
            serde_json::from_value(value).expect("Cache result didn't match type");
        Some(data)
    }

    fn read_entity(
        &self,
        entity_key: &String,
        selection: &Vec<FieldSelector>,
        dependencies: &mut HashSet<String>
    ) -> Option<serde_json::Value> {
        if entity_key != "Query" {
            dependencies.insert(entity_key.clone());
        }

        let guard = epoch::pin();
        let mut result = serde_json::Map::new();
        for field in selection {
            match field {
                &FieldSelector::Scalar(ref field_name, ref args) => {
                    let value = self.data.read_record(
                        entity_key,
                        &format!("{}{}", field_name, args),
                        &guard
                    )?;
                    result.insert(field_name.clone(), value.clone());
                }
                &FieldSelector::Object(ref field_name, ref args, _, ref inner) => {
                    let link = self.data.read_link(
                        entity_key,
                        &format!("{}{}", field_name, args),
                        &guard
                    )?;
                    match link {
                        Link::Single(ref entity_key) => {
                            let value = self.read_entity(entity_key, inner, dependencies)?;
                            result.insert(field_name.clone(), value);
                        }
                        Link::List(ref entity_keys) => {
                            let values: Option<Vec<_>> = entity_keys
                                .iter()
                                .map(|entity_key| self.read_entity(entity_key, inner, dependencies))
                                .collect();
                            result.insert(field_name.clone(), values?.into());
                        }
                        Link::Null => {
                            result.insert(field_name.clone(), Value::Null);
                        }
                    }
                }
                &FieldSelector::Union(ref field_name, ref args, _, ref inner) => {
                    let field_key = format!("{}{}", field_name, args);
                    let link = self.data.read_link(entity_key, &field_key, &guard)?;
                    match link {
                        Link::Single(ref entity_key) => {
                            let typename = self.data.read_record(entity_key, &field_key, &guard)?;
                            let typename = typename
                                .as_str()
                                .expect("__typename has invalid type! Should be string");
                            let selection = inner(typename);
                            let value = self.read_entity(entity_key, &selection, dependencies)?;
                            result.insert(field_name.clone(), value);
                        }
                        Link::List(ref entity_keys) => {
                            let values: Option<Vec<_>> = entity_keys
                                .iter()
                                .map(|entity_key| {
                                    let typename =
                                        self.data.read_record(entity_key, &field_key, &guard)?;
                                    let typename = typename
                                        .as_str()
                                        .expect("__typename has invalid type! Should be string");
                                    let selection = inner(typename);
                                    self.read_entity(entity_key, &selection, dependencies)
                                })
                                .collect();
                            result.insert(field_name.clone(), values?.into());
                        }
                        Link::Null => {
                            result.insert(field_name.clone(), Value::Null);
                        }
                    }
                }
            }
        }
        Some(result.into())
    }

    fn invalidate_union(
        &self,
        optimistic_key: Option<u64>,
        entity_key: &String,
        subselection: &Box<dyn Fn(&str) -> Vec<FieldSelector>>,
        invalidated: &mut HashSet<String>,
        guard: &Guard
    ) {
        let typename = self
            .data
            .read_record(entity_key, &TYPENAME, guard)
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
        dependencies: &mut HashSet<String>
    ) {
        if result.response.data.is_none() {
            return;
        }
        let key = result.meta.operation_type.to_string();

        let selection = Q::selection(variables);
        let guard = epoch::pin();
        let optimistic_key = if optimistic {
            Some(result.meta.query_key.clone())
        } else {
            None
        };

        self.invalidate_selection(optimistic_key, &key, &selection, dependencies, &guard);

        if !optimistic {
            self.data.clear_optimistic_layer(&result.meta.query_key);
        }
    }

    pub fn rerun_queries<C: Client>(
        &self,
        entities: HashSet<String>,
        originating_query: u64,
        client: &C
    ) {
        let queries: HashSet<_> = entities
            .iter()
            .flat_map(|entity| self.data.get_dependencies(entity))
            .collect();
        for query in queries {
            if query != originating_query {
                client.rerun_query(query);
            }
        }
    }

    fn invalidate_selection(
        &self,
        optimistic_key: Option<u64>,
        entity_key: &String,
        selection: &Vec<FieldSelector>,
        invalidated: &mut HashSet<String>,
        guard: &Guard
    ) {
        if entity_key != "Mutation" {
            invalidated.insert(entity_key.clone());
        }
        for field in selection {
            match field {
                &FieldSelector::Scalar(ref field_name, ref args) => {
                    self.write_record(
                        optimistic_key,
                        entity_key.clone(),
                        format!("{}{}", field_name, args),
                        None
                    );
                }
                &FieldSelector::Object(ref field_name, ref args, _, ref subselection) => {
                    if let Some(link) =
                        self.data
                            .read_link(entity_key, &format!("{}{}", field_name, args), &guard)
                    {
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
                &FieldSelector::Union(ref field_name, ref args, _, ref subselection) => {
                    if let Some(link) =
                        self.data
                            .read_link(entity_key, &format!("{}{}", field_name, args), &guard)
                    {
                        match link {
                            Link::Single(ref entity_key) => self.invalidate_union(
                                optimistic_key,
                                entity_key,
                                subselection,
                                invalidated,
                                guard
                            ),
                            Link::List(ref entity_keys) => {
                                for entity_key in entity_keys {
                                    self.invalidate_union(
                                        optimistic_key.clone(),
                                        entity_key,
                                        subselection,
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
