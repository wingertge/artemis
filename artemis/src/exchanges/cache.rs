use crate::{
    exchanges::Client,
    types::{ExchangeResult, Operation, OperationOptions, OperationResult},
    DebugInfo, Exchange, ExchangeFactory, GraphQLQuery, OperationMeta, OperationType, QueryError,
    RequestPolicy, Response, ResultSource
};
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex}
};
use surf::options;

type ResultCache = Arc<Mutex<HashMap<u64, Box<dyn Any + Send>>>>;
type OperationCache = Arc<Mutex<HashMap<&'static str, HashSet<u64>>>>;

pub struct CacheExchange;
impl<TNext: Exchange> ExchangeFactory<TNext> for CacheExchange {
    type Output = CacheExchangeImpl<TNext>;

    fn build(self, next: TNext) -> Self::Output {
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
        let OperationMeta { operation_type, .. } = &operation.meta;
        let key = operation.key;
        let request_policy = &operation.options.request_policy;

        operation_type == &OperationType::Query
            && request_policy != &RequestPolicy::NetworkOnly
            && (request_policy == &RequestPolicy::CacheOnly
                || self.result_cache.lock().unwrap().contains_key(&key))
    }

    fn after_query<Q: GraphQLQuery>(
        &self,
        operation_result: OperationResult<Q::ResponseData>
    ) -> Result<OperationResult<Q::ResponseData>, QueryError> {
        if operation_result.response.data.is_none() {
            return Ok(operation_result);
        }

        let OperationMeta { involved_types, .. } = &operation_result.meta;
        let key = operation_result.key;

        {
            let mut result_cache = self.result_cache.lock().unwrap();
            let data = operation_result.response.data.as_ref().unwrap().clone();
            result_cache.insert(key, Box::new(data));
        }
        {
            let mut operation_cache = self.operation_cache.lock().unwrap();
            for involved_type in involved_types {
                operation_cache
                    .entry(*involved_type)
                    .and_modify(|entry| {
                        entry.insert(key);
                    })
                    .or_insert_with(|| {
                        let mut set = HashSet::with_capacity(1);
                        set.insert(key);
                        set
                    });
            }
        }

        Ok(operation_result)
    }

    fn after_mutation<Q: GraphQLQuery, C: Client>(
        &self,
        operation_result: OperationResult<Q::ResponseData>,
        client: C
    ) -> Result<OperationResult<Q::ResponseData>, QueryError> {
        if operation_result.response.data.is_none() {
            return Ok(operation_result);
        }

        let OperationMeta { involved_types, .. } = &operation_result.meta;
        let key = operation_result.key;

        let ops_to_remove: HashSet<u64> = {
            let cache = self.operation_cache.lock().unwrap();
            let mut ops = HashSet::new();
            for involved_type in involved_types {
                let ops_for_type = cache.get(involved_type);
                if let Some(ops_for_type) = ops_for_type {
                    ops.extend(ops_for_type)
                }
            }
            ops.insert(key);
            ops
        };
        {
            let mut cache = self.result_cache.lock().unwrap();
            for op in ops_to_remove.iter() {
                cache.remove(op);
            }
        }
        for op in ops_to_remove {
            client.rerun_query(op);
        }
        Ok(operation_result)
    }
}

#[async_trait]
impl<TNext: Exchange> Exchange for CacheExchangeImpl<TNext> {
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        operation: Operation<Q::Variables>,
        client: C
    ) -> ExchangeResult<Q::ResponseData> {
        if should_skip::<Q>(&operation) {
            return self.next.run::<Q, _>(operation, client).await;
        }

        if !self.is_operation_cached::<Q>(&operation) {
            let res = self.next.run::<Q, _>(operation, client.clone()).await?;

            match res.meta.operation_type {
                OperationType::Query => self.after_query::<Q>(res),
                OperationType::Mutation => self.after_mutation::<Q, _>(res, client),
                _ => Ok(res)
            }
        } else {
            let key = &operation.key;

            let cached_result = {
                let cache = self.result_cache.lock().unwrap();
                cache.get(key).map(|res| {
                    let res: &Q::ResponseData = (&**res).downcast_ref::<Q::ResponseData>().unwrap();
                    res.clone()
                })
            };

            if let Some(cached) = cached_result {
                let result = OperationResult {
                    key: operation.key,
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
                self.next.run::<Q, _>(operation, client).await
            }
        }
    }
}
