use crate::types::{Operation, OperationResult};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::{Arc, Mutex}
};
use crate::{ResultSource, MiddlewareFactory, Middleware, OperationMeta, OperationType, RequestPolicy, DebugInfo, Response};

type ResultCache = Arc<Mutex<HashMap<u32, OperationResult>>>;
type OperationCache = Arc<Mutex<HashMap<&'static str, HashSet<u32>>>>;

pub struct CacheMiddleware<TNext: Middleware + Send + Sync> {
    result_cache: ResultCache,
    operation_cache: OperationCache,

    next: TNext
}

fn should_skip<T: Serialize + Send + Sync>(operation: &Operation<T>) -> bool {
    let operation_type = &operation.meta.operation_type;
    operation_type != &OperationType::Query && operation_type != &OperationType::Mutation
}

impl<TNext: Middleware + Send + Sync> CacheMiddleware<TNext> {
    fn is_operation_cached<T: Serialize + Send + Sync>(
        &self,
        operation: &Operation<T>
    ) -> bool {
        let OperationMeta {
            key,
            operation_type,
            ..
        } = &operation.meta;

        operation_type == &OperationType::Query
            && operation.request_policy != RequestPolicy::NetworkOnly
            && (operation.request_policy == RequestPolicy::CacheOnly
                || self.result_cache.lock().unwrap().contains_key(&key))
    }

    fn after_query(&self, operation_result: OperationResult) -> Result<OperationResult, Box<dyn Error>> {
        if operation_result.response.data.is_none() {
            return Ok(operation_result);
        }

        let OperationMeta { key, involved_types, .. } = &operation_result.meta;

        {
            let mut result_cache = self.result_cache.lock().unwrap();
            result_cache.insert(key.clone(), operation_result.clone());
        }
        {
            let mut operation_cache = self.operation_cache.lock().unwrap();
            for involved_type in involved_types {
                let involved_type = involved_type.clone();
                operation_cache.entry(involved_type)
                    .and_modify(|entry| {
                        entry.insert(key.clone());
                    })
                    .or_insert_with(|| {
                        let mut set = HashSet::with_capacity(1);
                        set.insert(key.clone());
                        set
                    });
            }
        }

        Ok(operation_result)
    }

    fn after_mutation(&self, operation_result: OperationResult) -> Result<OperationResult, Box<dyn Error>> {
        if operation_result.response.data.is_none() {
            return Ok(operation_result);
        }

        let OperationMeta {key, involved_types, ..} = &operation_result.meta;

        let ops_to_remove: HashSet<u32> = {
            let cache = self.operation_cache.lock().unwrap();
            let mut ops = HashSet::new();
            for involved_type in involved_types {
                let ops_for_type = cache.get(involved_type);
                if let Some(ops_for_type) = ops_for_type {
                    ops.extend(ops_for_type)
                }
            }
            ops.insert(key.clone());
            ops
        };
        {
            let mut cache = self.result_cache.lock().unwrap();
            for op in ops_to_remove {
                cache.remove(&op);
            }
        }
        Ok(operation_result)
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
    ) -> Result<OperationResult, Box<dyn Error + 'static>> {
        if should_skip(&operation) {
            return self.next.run(operation).await;
        }

        if !self.is_operation_cached(&operation) {
            let res = self.next.run(operation).await?;

            match &res.meta.operation_type {
                &OperationType::Query => self.after_query(res),
                &OperationType::Mutation => self.after_mutation(res),
                _ => Ok(res)
            }
        } else {
            let OperationMeta { key, .. } = &operation.meta;

            let cached_result = {
                let cache = self.result_cache.lock().unwrap();
                let cached_result = cache.get(key);
                let debug_info = Some(DebugInfo { // TODO: Make this conditional
                    source: ResultSource::Cache
                });

                cached_result.cloned().map(|cached_result| {
                    let OperationResult { meta, response } = cached_result.clone();
                    OperationResult {
                        meta,
                        response: Response {
                            debug_info,
                            ..response
                        }
                    }
                })
            };

            if let Some(cached) = cached_result {
                Ok(cached)
            } else {
                self.next.run(operation).await
            }
        }
    }
}
