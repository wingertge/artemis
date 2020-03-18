use crate::{
    store::Store,
    types::{NormalizedCacheExtension, NormalizedCacheOptions}
};
use artemis::{
    exchanges::Client, DebugInfo, Exchange, ExchangeFactory, ExchangeResult, GraphQLQuery,
    Operation, OperationResult, OperationType, QueryError, RequestPolicy, Response, ResultSource
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc
};
use std::num::Wrapping;
use serde_json::Value;
use parking_lot::{Mutex, RwLock};
use wasm_bindgen::JsValue;

#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct NormalizedCacheExchange {
    options: Option<NormalizedCacheOptions>
}

impl NormalizedCacheExchange {
    #[allow(unused)]
    pub fn with_options(options: NormalizedCacheOptions) -> Self {
        Self {
            options: Some(options)
        }
    }

    #[allow(unused)]
    pub fn new() -> Self {
        Self { options: None }
    }
}

impl<TNext: Exchange> ExchangeFactory<TNext> for NormalizedCacheExchange
{
    type Output = NormalizedCacheImpl<TNext>;

    fn build(self, next: TNext) -> NormalizedCacheImpl<TNext> {
        let options = self
            .options
            .unwrap_or_else(|| NormalizedCacheOptions::default());
        let store = Store::new(options.custom_keys.unwrap_or_else(|| HashMap::new()));
        NormalizedCacheImpl {
            next,
            store: Arc::new(store),
            #[cfg(target_arch = "wasm32")]
            updaters: HashMap::new()
        }
    }
}

pub(crate) struct NormalizedCacheImpl<TNext: Exchange> {
    next: TNext,
    store: Arc<Store>,
    //#[cfg(target_arch = "wasm32")]
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

//#[cfg(target_arch = "wasm32")]
pub fn hash_query(x: &str) -> u64 {
    let x = x.as_bytes();
    let mut h = Wrapping(5381u64);
    for i in 0..x.len() {
        h = (h << 5) + h + Wrapping(x[i] as u64)
    }

    h.0
}

impl<TNext: Exchange> NormalizedCacheImpl<TNext> {
    fn write_updater<Q: GraphQLQuery>(&self, operation: &Operation<Q::Variables>) {
        let hash = hash_query(operation.query.query);
        if self.updaters.read().contains_key(&hash) {
            let updater = {
                let store = self.store.clone();
                move |variables: Value, updater_fn: js_sys::Function, dependencies: *mut usize| {
                    store.update_query_js::<Q>(variables, updater_fn, dependencies);
                }
            };
            self.updaters.write().insert(hash, Box::new(updater));
        }
    }

    fn after_query<Q: GraphQLQuery, C: Client>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        client: &C
    ) -> Result<(), QueryError> {
        let query_key = result.meta.query_key.clone();
        let mut dependencies = HashSet::new();
        if result.response.errors.is_some() {
            self.store.clear_optimistic_layer(&query_key);
        } else {
            self.store
                .write_query::<Q>(result, variables, false, &mut dependencies)?;
            self.store.rerun_queries(dependencies, query_key, client);
            self.store.clear_optimistic_layer(&query_key);
        }
        Ok(())
    }

    fn after_mutation<Q: GraphQLQuery, C: Client>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: Q::Variables,
        client: &C,
        extension: Option<&NormalizedCacheExtension>
    ) {
        let query_key = result.meta.query_key.clone();
        let mut dependencies = HashSet::new();
        self.store
            .invalidate_query::<Q>(result, &variables, false, &mut dependencies);
        self.store
            .write_query::<Q>(result, &variables, false, &mut dependencies)
            .unwrap();
        if let Some(updater) = extension.and_then(|ext| ext.update.as_ref()) {
            updater(
                &result.response.data,
                self.store.clone().into(),
                &mut dependencies
            );
        } else if let Some(updater) = extension.and_then(|ext| ext.update_js.as_ref()) {
            if cfg!(target_arch = "wasm32") {
                self.update_js(&result.response.data, updater, &mut dependencies);
            }
        }
        self.store.rerun_queries(dependencies, query_key, client);
    }

    fn update_js<Q: GraphQLQuery>(&self, data: &Q::ResponseData, updater: js_sys::Function, dependencies: &mut HashSet<String>) {
        let this = JsValue::NULL;
        let data = serde_wasm_bindgen::to_value(data).unwrap();
        let dependencies = dependencies as *mut _ as *mut usize;
        updater.call3(&this, &data, &|query: u64, variables: JsValue, updater: js_sys::Function, dependencies: *mut usize| {
            let updaters = self.updaters.read();
            let updater_fn = updaters.get(&query);
            if let Some(updater_fn) = updater_fn {
                updater_fn(variables, updater, dependencies);
            }
        }, dependencies)
    }

    fn on_mutation<Q: GraphQLQuery, C: Client>(
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
                    meta: operation.meta.clone(),
                    response: Response {
                        data: Some(data),
                        debug_info: None,
                        errors: None
                    }
                };

                let query_key = operation.meta.query_key.clone();
                let mut dependencies = HashSet::new();
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
                } else if let Some(updater) = extension.and_then(|ext| ext.update_js.as_ref()) {
                    if cfg!(target_arch = "wasm32") {
                        self.update_js(&result.response.data, updater, &mut dependencies);
                    }
                }
                println!("Optimistic dependencies: {:?}", dependencies);
                self.store.rerun_queries(dependencies, query_key, client);
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
            .and_then(|ext| ext.get::<NormalizedCacheExtension>("NormalizedCache"));
        let extension = extension.as_ref();

        if should_cache::<Q>(&operation) {
            self.write_updater(&operation);
            let mut deps = HashSet::new();
            if let Some(cached) = self.store.read_query::<Q>(&operation, &mut deps) {
                let response = OperationResult {
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
                let variables: Q::Variables = operation.query.variables.clone();
                let res = self.next.run::<Q, _>(operation, client.clone()).await?;
                self.after_query::<Q, _>(&res, &variables, &client)?;
                Ok(res)
            }
        } else {
            let operation_type = operation.meta.operation_type.clone();

            if operation_type == OperationType::Mutation {
                self.on_mutation::<Q, _>(&operation, &client, extension);
            }

            let variables = operation.query.variables.clone();
            let res = self
                .next
                .run::<Q, _>(operation.clone(), client.clone())
                .await?;
            if operation_type == OperationType::Mutation {
                self.after_mutation::<Q, _>(&res, variables, &client, extension);
            }
            Ok(res)
        }
    }
}
