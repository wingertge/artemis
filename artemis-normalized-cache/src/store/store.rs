use crate::store::data::{InMemoryData, Link};
use artemis::{
    exchanges::Client, FieldSelector, GraphQLQuery, Operation, OperationResult, QueryError,
    QueryInfo
};
use flurry::{epoch, epoch::Guard};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt
};

pub struct Store {
    custom_keys: HashMap<&'static str, String>,
    data: InMemoryData
}

#[derive(Debug)]
pub enum StoreError {
    InvalidSelection(String),
    InvalidMetadata(String)
}
impl Error for StoreError {}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::InvalidSelection(msg) => write!(f, "Invalid selection: {}", msg),
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

impl Store {
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

    /*    pub fn update_query<Q: GraphQLQuery, F>(&self, query: QueryBody<Q::Variables>, updater: F)
    where
        F: Fn(Option<serde_json::Value>) -> serde_json::Value
    {
    }*/

    pub fn write_query<Q: GraphQLQuery, C: Client>(
        &self,
        query: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        optimistic: bool,
        client: &C
    ) -> Result<(), QueryError> {
        if query.response.data.is_none() {
            return Ok(());
        }

        let data: Q::ResponseData = query.response.data.as_ref().unwrap().clone();
        let typename: &str = data.typename();
        let selection = Q::selection(variables);
        let data = serde_json::to_value(data)?;
        if !data.is_object() {
            return Ok(());
        }
        let data = data.as_object().unwrap();
        let key = self.key_of_entity(&typename, data).ok_or_else(|| {
            StoreError::InvalidMetadata(format!(
                "Cache error: couldn't find root key of query {}",
                query.meta.key
            ))
        })?;
        let mut dependencies = HashSet::new();

        let guard = epoch::pin();
        let optimistic_key = if optimistic {
            Some(query.meta.key.clone())
        } else {
            None
        };

        for (field_name, value) in data {
            let args = selection
                .iter()
                .find_map(|selector| {
                    let (name, args) = match selector {
                        &FieldSelector::Scalar(ref name, ref args) => (name, args),
                        &FieldSelector::Object(ref name, ref args, _, _) => (name, args),
                        &FieldSelector::Union(ref name, ref args, _, _) => (name, args)
                    };
                    if name == field_name {
                        Some(args)
                    } else {
                        None
                    }
                })
                .expect(&format!(
                    "Missing selector for returned field {}:{}",
                    typename, field_name
                ));
            self.store_data(
                optimistic_key,
                key.clone(),
                field_name,
                args,
                value.clone(),
                &selection,
                &mut dependencies,
                &guard
            )?;
        }

        if !optimistic {
            self.data
                .set_dependencies(query.meta.key.clone(), dependencies.clone());
            self.data.clear_optimistic_layer(&query.meta.key);
        }

        self.rerun_queries(dependencies, query.meta.key.clone(), client);
        self.data
            .set_entity_key_for_query(query.meta.key.clone(), key.clone());

        Ok(())
    }

    fn store_object(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field_name: &String,
        args: &String,
        value: serde_json::Map<String, serde_json::Value>,
        selection: &Vec<FieldSelector>,
        dependencies: &mut HashSet<String>,
        guard: &Guard
    ) -> Result<(), QueryError> {
        let inner_selector = self.find_selector(selection, &entity_key, field_name, guard)?;
        let key = self.key_of_entity(&"TODO", &value).ok_or_else(|| {
            StoreError::InvalidMetadata(format!(
                "Cache error: couldn't find index for {}:{}",
                entity_key, field_name
            ))
        })?;
        dependencies.insert(key.clone());
        for (field_name, value) in value.into_iter() {
            let args = inner_selector
                .iter()
                .find_map(|selector| {
                    let (name, args) = match selector {
                        &FieldSelector::Scalar(ref name, ref args) => (name, args),
                        &FieldSelector::Object(ref name, ref args, _, _) => (name, args),
                        &FieldSelector::Union(ref name, ref args, _, _) => (name, args)
                    };
                    if name == &field_name {
                        Some(args)
                    } else {
                        None
                    }
                })
                .expect(&format!(
                    "Missing selector for returned field {}:{}",
                    "TODO", field_name
                ));
            self.store_data(
                optimistic_key.clone(),
                key.clone(),
                &field_name,
                args,
                value,
                &inner_selector,
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

    fn store_array(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field_name: &String,
        args: &String,
        values: Vec<serde_json::Value>,
        selection: &Vec<FieldSelector>,
        dependencies: &mut HashSet<String>,
        guard: &Guard
    ) -> Result<(), QueryError> {
        let field_key = format!("{}{}", field_name, args);
        if values.len() == 0 {
            Ok(())
        } else if !values.iter().next().unwrap().is_object() {
            self.write_record(optimistic_key, entity_key, field_key, Some(values.into()));
            Ok(())
        } else {
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

                let inner_selector = self.find_selector(selection, &key, field_name, guard)?;

                for (field_name, value) in value {
                    self.store_data(
                        optimistic_key.clone(),
                        key.clone(),
                        field_name,
                        args,
                        value.clone().into(),
                        &inner_selector,
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
    }

    fn find_selector(
        &self,
        selection: &Vec<FieldSelector>,
        entity_key: &String,
        field_name: &String,
        guard: &Guard
    ) -> Result<Vec<FieldSelector>, QueryError> {
        Ok(selection
            .iter()
            .find_map(|selector| {
                if let FieldSelector::Object(name, _, _, inner) = selector {
                    if name == field_name {
                        Some(inner.clone())
                    } else {
                        None
                    }
                } else if let FieldSelector::Union(name, _, _, inner) = selector {
                    if name == field_name {
                        let typename = self
                            .data
                            .read_record(entity_key, &TYPENAME, guard)
                            .expect("Missing typename from union type. This is a codegen error.");
                        let typename = typename
                            .as_str()
                            .expect("__typename has the wrong type. Should be string.");
                        Some(inner(typename))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or(StoreError::InvalidSelection(format!(
                "Couldn't find returned field in selection"
            )))?)
    }

    fn store_data(
        &self,
        optimistic_key: Option<u64>,
        entity_key: String,
        field_name: &String,
        args: &String,
        data: serde_json::Value,
        selection: &Vec<FieldSelector>,
        dependencies: &mut HashSet<String>,
        guard: &Guard
    ) -> Result<(), QueryError> {
        let field_key = format!("{}{}", field_name, args);
        if let Some(values) = data.as_array() {
            self.store_array(
                optimistic_key,
                entity_key,
                field_name,
                args,
                values.clone(),
                selection,
                dependencies,
                guard
            )?;
        } else if let Some(value) = data.as_object() {
            self.store_object(
                optimistic_key,
                entity_key,
                field_name,
                args,
                value.clone(),
                selection,
                dependencies,
                guard
            )?;
        } else {
            self.write_record(optimistic_key, entity_key, field_key, Some(data));
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
        query: &Operation<Q::Variables>
    ) -> Option<Q::ResponseData> {
        let guard = epoch::pin();
        let root_key = self.data.get_entity_key_for_query(&query.meta.key, &guard);
        if let Some(root_key) = root_key {
            let selection = Q::selection(&query.query.variables);
            let value = self.read_entity(root_key, &selection)?;
            let data: Q::ResponseData =
                serde_json::from_value(value).expect("Cache result didn't match type");
            Some(data)
        } else {
            None
        }
    }

    fn read_entity(
        &self,
        entity_key: &String,
        selection: &Vec<FieldSelector>
    ) -> Option<serde_json::Value> {
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
                &FieldSelector::Object(ref field_name, ref args, optional, ref inner) => {
                    let link =
                        self.data
                            .read_link(entity_key, &format!("{}{}", field_name, args), &guard);
                    if let Some(link) = link {
                        match link {
                            Link::Single(ref entity_key) => {
                                let value = self.read_entity(entity_key, inner)?;
                                result.insert(field_name.clone(), value);
                            }
                            Link::List(ref entity_keys) => {
                                let values: Option<Vec<_>> = entity_keys
                                    .iter()
                                    .map(|entity_key| self.read_entity(entity_key, inner))
                                    .collect();
                                result.insert(field_name.clone(), values?.into());
                            }
                        }
                    } else if optional {
                        result.insert(field_name.clone(), serde_json::Value::Null);
                    }
                }
                &FieldSelector::Union(ref field_name, ref args, optional, ref inner) => {
                    let field_key = format!("{}{}", field_name, args);
                    let link = self.data.read_link(entity_key, &field_key, &guard);
                    if let Some(link) = link {
                        match link {
                            Link::Single(ref entity_key) => {
                                let typename =
                                    self.data.read_record(entity_key, &field_key, &guard)?;
                                let typename = typename
                                    .as_str()
                                    .expect("__typename has invalid type! Should be string");
                                let selection = inner(typename);
                                let value = self.read_entity(entity_key, &selection)?;
                                result.insert(field_name.clone(), value);
                            }
                            Link::List(ref entity_keys) => {
                                let values: Option<Vec<_>> = entity_keys
                                    .iter()
                                    .map(|entity_key| {
                                        let typename = self
                                            .data
                                            .read_record(entity_key, &field_key, &guard)?;
                                        let typename = typename.as_str().expect(
                                            "__typename has invalid type! Should be string"
                                        );
                                        let selection = inner(typename);
                                        self.read_entity(entity_key, &selection)
                                    })
                                    .collect();
                                result.insert(field_name.clone(), values?.into());
                            }
                        }
                    } else if optional {
                        result.insert(field_name.clone(), serde_json::Value::Null);
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

    pub fn invalidate_query<Q: GraphQLQuery, C: Client>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        client: &C,
        optimistic: bool
    ) {
        if result.response.data.is_none() {
            return;
        }
        let data: Q::ResponseData = result.response.data.as_ref().unwrap().clone();

        let typename = QueryInfo::<Q::Variables>::typename(&data);
        let data = serde_json::to_value(data).unwrap();
        let data = data.as_object().unwrap();
        let key = self
            .key_of_entity(typename, data)
            .expect(&format!("Failed to find key for {}", typename));
        let mut invalidated = HashSet::new();
        invalidated.insert(key.clone());

        let selection = Q::selection(variables);
        let guard = epoch::pin();
        let optimistic_key = if optimistic {
            Some(result.meta.key.clone())
        } else {
            None
        };

        self.invalidate_selection(optimistic_key, &key, &selection, &mut invalidated, &guard);

        if !optimistic {
            self.data.clear_optimistic_layer(&result.meta.key);
        }

        self.rerun_queries(invalidated, result.meta.key.clone(), client);
    }

    fn rerun_queries<C: Client>(
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
        invalidated.insert(entity_key.clone());
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
                        }
                    }
                }
            }
        }
    }
}
