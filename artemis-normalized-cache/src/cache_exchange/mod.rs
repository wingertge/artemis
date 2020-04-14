//! Contains the exchange factory and implementation. The factory is the only thing needed for most
//! users and is reexported from the root.

use crate::{
    store::Store,
    types::{NormalizedCacheExtension, NormalizedCacheOptions}
};
use artemis::{
    exchange::{
        Client, Exchange, ExchangeFactory, ExchangeResult, Operation, OperationResult,
        OperationType
    },
    DebugInfo, GraphQLQuery, QueryError, RequestPolicy, Response, ResultSource
};
#[cfg(target_arch = "wasm32")]
use parking_lot::RwLock;
#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;
use std::{
    collections::{HashMap},
    sync::Arc
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(test)]
mod tests;

/// The normalized cache exchange. This will store normalized queries by unique ID.
#[derive(Default)]
pub struct NormalizedCacheExchange {
    options: Option<NormalizedCacheOptions>
}

impl NormalizedCacheExchange {
    /// Create a new cache exchange with extra options.
    #[allow(unused)]
    pub fn with_options(options: NormalizedCacheOptions) -> Self {
        Self {
            options: Some(options)
        }
    }

    /// Create a new cache exchange with default options
    #[allow(unused)]
    pub fn new() -> Self {
        Self { options: None }
    }
}

impl<TNext: Exchange> ExchangeFactory<TNext> for NormalizedCacheExchange {
    type Output = NormalizedCacheImpl<TNext>;

    fn build(self, next: TNext) -> NormalizedCacheImpl<TNext> {
        let options = self.options.unwrap_or_else(NormalizedCacheOptions::default);
        let store = Store::new(options.custom_keys.unwrap_or_else(HashMap::new));
        NormalizedCacheImpl {
            next,
            store: Arc::new(store),
            #[cfg(target_arch = "wasm32")]
            updaters: HashMap::new()
        }
    }
}

/// The implementation of the normalized cache. Exposed in case someone needs it, but most users
/// shouldn't.
pub struct NormalizedCacheImpl<TNext: Exchange> {
    next: TNext,
    store: Arc<Store>,
    #[cfg(target_arch = "wasm32")]
    updaters: Arc<RwLock<HashMap<u64, Box<dyn Fn(JsValue, js_sys::Function, *mut usize)>>>>
}

fn should_cache<Q: GraphQLQuery>(operation: &Operation<Q::Variables>) -> bool {
    operation.meta.operation_type == OperationType::Query
        && operation.options.request_policy != RequestPolicy::NetworkOnly
}

fn is_optimistic_mutation<Q: GraphQLQuery>(op: &Operation<Q::Variables>) -> bool {
    op.meta.operation_type == OperationType::Mutation
        && op.options.request_policy != RequestPolicy::NetworkOnly
}

impl<TNext: Exchange> NormalizedCacheImpl<TNext> {
    #[cfg(target_arch = "wasm32")]
    fn write_updater<Q: GraphQLQuery>(&self, operation: &Operation<Q::Variables>)
    where
        Q::Variables: DeserializeOwned
    {
        if self.updaters.read().contains_key(&operation.key) {
            let updater = {
                let store = self.store.clone();
                move |variables: Value, updater_fn: js_sys::Function, dependencies: *mut usize| {
                    store.update_query_js::<Q>(variables, updater_fn, dependencies);
                }
            };
            self.updaters
                .write()
                .insert(operation.key, Box::new(updater));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn write_updater<Q: GraphQLQuery>(&self, _operation: &Operation<Q::Variables>) {}

    fn write_query<Q: GraphQLQuery, C: Client>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        client: &C
    ) -> Result<(), QueryError> {
        let query_key = result.key;
        let mut dependencies = Vec::with_capacity(10);
        if result.response.errors.is_some() {
            self.store.clear_optimistic_layer(query_key);
        } else {
            self.store
                .write_query::<Q>(result, variables, false, &mut dependencies)?;
            self.store.rerun_queries(dependencies, query_key, client);
            self.store.clear_optimistic_layer(query_key);
        }
        Ok(())
    }

    fn invalidate_and_update<Q: GraphQLQuery, C: Client>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: Q::Variables,
        client: &C,
        extension: Option<&NormalizedCacheExtension>
    ) {
        let query_key = result.key;
        let mut dependencies = Vec::with_capacity(10);
        self.store
            .invalidate_query::<Q>(result, &variables, false, &mut dependencies);
        if let Some(updater) = extension.and_then(|ext| ext.update.as_ref()) {
            updater(
                &result.response.data,
                self.store.clone().into(),
                &mut dependencies
            );
        } else {
            self.update_js::<Q>(extension, result.response.data.as_ref(), &mut dependencies);
        }
        self.store.rerun_queries(dependencies, query_key, client);
    }

    #[cfg(target_arch = "wasm32")]
    fn update_js<Q: GraphQLQuery>(
        &self,
        extension: Option<&NormalizedCacheExtension>,
        data: Option<&Q::ResponseData>,
        dependencies: &mut Vec<String>
    ) {
        if let Some(updater) = extension.and_then(|ext| ext.update_js) {
            let this = JsValue::NULL;
            let data = serde_wasm_bindgen::to_value(data).unwrap();
            let dependencies = dependencies as *mut _ as *mut usize;
            updater.call3(
                &this,
                &data,
                &|query: u64,
                  variables: JsValue,
                  updater: js_sys::Function,
                  dependencies: *mut usize| {
                    let updaters = self.updaters.read();
                    let updater_fn = updaters.get(&query);
                    if let Some(updater_fn) = updater_fn {
                        updater_fn(variables, updater, dependencies);
                    }
                },
                dependencies
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn update_js<Q: GraphQLQuery>(
        &self,
        _extension: Option<&NormalizedCacheExtension>,
        _data: Option<&Q::ResponseData>,
        _dependencies: &mut Vec<String>
    ) {
    }

    fn run_optimistic_query<Q: GraphQLQuery, C: Client>(
        &self,
        operation: &Operation<Q::Variables>,
        client: &C,
        extension: Option<&NormalizedCacheExtension>
    ) {
        if operation.options.request_policy != RequestPolicy::NetworkOnly {
            let variables = &operation.query.variables;
            let data = extension
                .and_then(|ext| ext.optimistic_result.as_ref())
                .and_then(|resolver| resolver())
                .and_then(|result| result.downcast::<Q::ResponseData>().ok())
                .map(|result| *result);

            if let Some(data) = data {
                let result = OperationResult {
                    key: operation.key,
                    meta: operation.meta.clone(),
                    response: Response {
                        data: Some(data),
                        debug_info: None,
                        errors: None
                    }
                };

                let query_key = operation.key;

                let mut dependencies = Vec::new();
                self.store
                    .write_query::<Q>(&result, variables, true, &mut dependencies)
                    .unwrap();

                println!("Optimistic dependencies: {:?}", dependencies);
                self.store.rerun_queries(dependencies, query_key, client);
                client.push_result(operation.key, Ok(result))
            }
        }
    }

    fn run_optimistic_mutation<Q: GraphQLQuery, C: Client>(
        &self,
        operation: &Operation<Q::Variables>,
        client: &C,
        extension: Option<&NormalizedCacheExtension>
    ) {
        if is_optimistic_mutation::<Q>(operation) {
            let variables = &operation.query.variables;
            let data = extension
                .and_then(|extension: &NormalizedCacheExtension| {
                    extension.optimistic_result.as_ref()
                })
                .and_then(|resolver| resolver())
                .and_then(|result| result.downcast::<Q::ResponseData>().ok())
                .map(|result| *result);

            if let Some(data) = data {
                let result = OperationResult {
                    key: operation.key,
                    meta: operation.meta.clone(),
                    response: Response {
                        data: Some(data),
                        debug_info: None,
                        errors: None
                    }
                };

                let query_key = operation.key;
                let mut dependencies = Vec::with_capacity(10);
                self.store
                    .invalidate_query::<Q>(&result, variables, true, &mut dependencies);
                self.store
                    .write_query::<Q>(&result, variables, true, &mut dependencies)
                    .unwrap();
                if let Some(updater) = extension.and_then(|ext| ext.update.as_ref()) {
                    updater(
                        &result.response.data,
                        self.store.clone().into(),
                        &mut dependencies
                    );
                } else {
                    self.update_js::<Q>(
                        extension,
                        result.response.data.as_ref(),
                        &mut dependencies
                    );
                }
                self.store.rerun_queries(dependencies, query_key, client);
                client.push_result(operation.key, Ok(result))
            }
        }
    }
}

#[async_trait]
impl<TNext: Exchange> Exchange for NormalizedCacheImpl<TNext> {
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        operation: Operation<Q::Variables>,
        client: C
    ) -> ExchangeResult<Q::ResponseData> {
        let extension = operation
            .options
            .extensions
            .as_ref()
            .and_then(|ext| ext.get::<NormalizedCacheExtension, _>("NormalizedCache"));
        let extension = extension.as_ref();

        if should_cache::<Q>(&operation) {
            self.write_updater::<Q>(&operation);
            let mut deps = Vec::with_capacity(10);
            if let Some(cached) = self.store.read_query::<Q>(&operation, &mut deps) {
                let response = OperationResult {
                    key: operation.key,
                    response: Response {
                        debug_info: Some(DebugInfo {
                            did_dedup: false,
                            source: ResultSource::Cache
                        }),
                        data: Some(cached),
                        errors: None
                    },
                    meta: operation.meta
                };
                Ok(response)
            } else {
                self.run_optimistic_query::<Q, _>(&operation, &client, extension);
                let variables: Q::Variables = operation.query.variables.clone();
                let res = self.next.run::<Q, _>(operation, client.clone()).await?;
                self.write_query::<Q, _>(&res, &variables, &client)?;
                Ok(res)
            }
        } else {
            let operation_type = operation.meta.operation_type.clone();

            if operation_type == OperationType::Mutation {
                self.run_optimistic_mutation::<Q, _>(&operation, &client, extension);
            }

            let variables = operation.query.variables.clone();
            let res = self
                .next
                .run::<Q, _>(operation.clone(), client.clone())
                .await?;
            if operation_type == OperationType::Mutation {
                self.invalidate_and_update::<Q, _>(&res, variables, &client, extension);
            }
            Ok(res)
        }
    }
}
