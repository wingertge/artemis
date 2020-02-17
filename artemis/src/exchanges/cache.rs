use crate::{types::{ExchangeResult, Operation, OperationResult}, Exchange, ExchangeFactory, OperationMeta, OperationType, RequestPolicy, Response, ResultSource, DebugInfo, GraphQLQuery};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    sync::{Arc, Mutex}
};
use std::any::Any;

type ResultCache = Arc<Mutex<HashMap<u64, Box<dyn Any + Send>>>>;
type OperationCache = Arc<Mutex<HashMap<&'static str, HashSet<u64>>>>;

pub struct CacheExchange;
impl<TNext: Exchange> ExchangeFactory<CacheExchangeImpl<TNext>, TNext> for CacheExchange {
    fn build(self, next: TNext) -> CacheExchangeImpl<TNext> {
        CacheExchangeImpl {
            result_cache: Arc::new(Mutex::new(HashMap::new())),
            operation_cache: Arc::new(Mutex::new(HashMap::new())),

            next
        }
    }
}

pub struct CacheExchangeImpl<TNext: Exchange> {
    result_cache: ResultCache,
    operation_cache: OperationCache,

    next: TNext
}

#[inline]
fn should_skip<Q: GraphQLQuery>(operation: &Operation<Q::Variables>) -> bool {
    let operation_type = &operation.meta.operation_type;
    operation_type != &OperationType::Query && operation_type != &OperationType::Mutation
}

impl<TNext: Exchange> CacheExchangeImpl<TNext> {
    fn is_operation_cached<Q: GraphQLQuery>(&self, operation: &Operation<Q::Variables>) -> bool {
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

    fn after_query<Q: GraphQLQuery>(
        &self,
        operation_result: OperationResult<Q::ResponseData>
    ) -> Result<OperationResult<Q::ResponseData>, Box<dyn Error>> {
        if operation_result.response.data.is_none() {
            return Ok(operation_result);
        }

        let OperationMeta {
            key,
            involved_types,
            ..
        } = &operation_result.meta;

        {
            let mut result_cache = self.result_cache.lock().unwrap();
            let data = operation_result.response.data.as_ref().unwrap().clone();
            result_cache.insert(key.clone(), Box::new(data));
        }
        {
            let mut operation_cache = self.operation_cache.lock().unwrap();
            for involved_type in involved_types {
                let involved_type = involved_type.clone();
                operation_cache
                    .entry(involved_type)
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

    fn after_mutation<Q: GraphQLQuery>(
        &self,
        operation_result: OperationResult<Q::ResponseData>
    ) -> Result<OperationResult<Q::ResponseData>, Box<dyn Error>> {
        if operation_result.response.data.is_none() {
            return Ok(operation_result);
        }

        let OperationMeta {
            key,
            involved_types,
            ..
        } = &operation_result.meta;

        let ops_to_remove: HashSet<u64> = {
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

#[async_trait]
impl<TNext: Exchange> Exchange for CacheExchangeImpl<TNext> {
    async fn run<Q: GraphQLQuery>(&self, operation: Operation<Q::Variables>) -> ExchangeResult<Q::ResponseData> {
        if should_skip::<Q>(&operation) {
            return self.next.run::<Q>(operation).await;
        }

        if !self.is_operation_cached::<Q>(&operation) {
            let res = self.next.run::<Q>(operation).await?;

            match &res.meta.operation_type {
                &OperationType::Query => self.after_query::<Q>(res),
                &OperationType::Mutation => self.after_mutation::<Q>(res),
                _ => Ok(res)
            }
        } else {
            let OperationMeta { key, .. } = &operation.meta;

            let cached_result = {
                let cache = self.result_cache.lock().unwrap();
                cache.get(key).map(|res| {
                    let res: &Q::ResponseData = (&**res)
                        .downcast_ref::<Q::ResponseData>()
                        .unwrap();
                    res.clone()
                })
            };

            if let Some(cached) = cached_result {
                let result = OperationResult {
                    meta: operation.meta,
                    response: Response {
                        debug_info: Some(DebugInfo {
                            source: ResultSource::Cache,
                            did_dedup: false
                        }),
                        data: Some(cached),
                        errors: None
                    }
                };
                Ok(result)
            } else {
                self.next.run::<Q>(operation).await
            }
        }
    }
}
