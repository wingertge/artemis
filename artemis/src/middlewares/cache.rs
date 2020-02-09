use crate::types::{
    Middleware, MiddlewareFactory, Operation, OperationMeta, OperationResult, OperationType,
    RequestPolicy
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::{Arc, Mutex}
};

type ResultCache = Arc<Mutex<HashMap<u32, OperationResult>>>;
type OperationCache = Arc<Mutex<HashMap<String, HashSet<u32>>>>;

#[derive(Default)]
struct CacheMiddleware<TNext: Middleware + Send + Sync> {
    result_cache: ResultCache,
    operation_cache: OperationCache,

    next: TNext
}

impl<TNext: Middleware + Send + Sync> CacheMiddleware<TNext> {
    fn after_mutation(&self, response: OperationResult) {}

    async fn is_operation_cached<T: Serialize + Send + Sync>(
        &self,
        operation: Operation<T>
    ) -> bool {
        let OperationMeta {
            key,
            operation_type,
            ..
        } = operation.meta;

        operation_type == OperationType::Query
            && operation.request_policy != RequestPolicy::NetworkOnly
            && (operation.request_policy == RequestPolicy::CacheOnly
                || self.result_cache.lock().unwrap().contains_key(&key))
    }
}

impl<TNext: Middleware + Send + Sync> MiddlewareFactory<CacheMiddleware<TNext>, TNext>
    for CacheMiddleware<TNext>
{
    fn build(next: TNext) -> CacheMiddleware<TNext> {
        Self {
            result_cache: Arc::new(Mutex::new(HashMap::new())),
            operation_cache: Arc::new(Mutex::new(HashMap::new())),

            next
        }
    }
}

#[async_trait]
impl<TNext: Middleware + Send + Sync> Middleware for CacheMiddleware<TNext> {
    async fn run<V: Serialize + Send + Sync>(
        &self,
        operation: Operation<V>
    ) -> Result<OperationResult, Box<dyn Error>> {
        self.next.run(operation).await
    }
}
