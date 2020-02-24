use crate::{
    store::Store,
    types::{NormalizedCacheExtension, NormalizedCacheOptions}
};
use artemis::{
    client::ClientImpl, DebugInfo, Exchange, ExchangeFactory, ExchangeResult, GraphQLQuery,
    Operation, OperationResult, OperationType, QueryError, RequestPolicy, Response, ResultSource
};
use std::{collections::HashMap, sync::Arc};

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

impl<TNext: Exchange> ExchangeFactory<NormalizedCacheImpl<TNext>, TNext>
    for NormalizedCacheExchange
{
    fn build(self, next: TNext) -> NormalizedCacheImpl<TNext> {
        let options = self
            .options
            .unwrap_or_else(|| NormalizedCacheOptions::default());
        let store = Store::new(options.custom_keys.unwrap_or_else(|| HashMap::new()));
        NormalizedCacheImpl {
            next,
            store: Arc::new(store)
        }
    }
}

pub(crate) struct NormalizedCacheImpl<TNext: Exchange> {
    next: TNext,
    store: Arc<Store>
}

fn should_cache<Q: GraphQLQuery>(operation: &Operation<Q::Variables>) -> bool {
    operation.meta.operation_type == OperationType::Query
        && operation.options.request_policy != RequestPolicy::NetworkOnly
}

fn is_optimistic_mutation<Q: GraphQLQuery>(op: &Operation<Q::Variables>) -> bool {
    op.meta.operation_type == OperationType::Mutation
        && op.options.request_policy != RequestPolicy::NetworkOnly
}

impl<TNext: Exchange> NormalizedCacheImpl<TNext> {}

impl<TNext: Exchange> NormalizedCacheImpl<TNext> {
    fn after_query<Q: GraphQLQuery>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables
    ) -> Result<(), QueryError> {
        self.store.write_query::<Q>(result, variables, false)
    }

    fn after_mutation<Q: GraphQLQuery, M: Exchange>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: Q::Variables,
        client: &Arc<ClientImpl<M>>
    ) {
        self.store
            .invalidate_query::<Q, M>(result, &variables, client, false);
        self.store
            .write_query::<Q>(result, &variables, false)
            .unwrap();
    }

    fn on_mutation<Q: GraphQLQuery, M: Exchange>(
        &self,
        operation: &Operation<Q::Variables>,
        client: &Arc<ClientImpl<M>>
    ) {
        if is_optimistic_mutation::<Q>(operation) {
            let variables = &operation.query.variables;
            let data = operation
                .options
                .extensions
                .as_ref()
                .and_then(|extensions| extensions.get::<NormalizedCacheExtension>())
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

                self.store
                    .invalidate_query::<Q, M>(&result, variables, client, true);
                self.store
                    .write_query::<Q>(&result, variables, true)
                    .unwrap();
            }
        }
    }
}

#[async_trait]
impl<TNext: Exchange> Exchange for NormalizedCacheImpl<TNext> {
    async fn run<Q: GraphQLQuery, M: Exchange>(
        &self,
        operation: Operation<Q::Variables>,
        client: Arc<ClientImpl<M>>
    ) -> ExchangeResult<Q::ResponseData> {
        if should_cache::<Q>(&operation) {
            if let Some(cached) = self.store.read_query::<Q>(&operation) {
                println!("Hit");
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
                println!("Miss");
                let variables: Q::Variables = operation.query.variables.clone();
                let res = self.next.run::<Q, M>(operation, client).await?;
                self.after_query::<Q>(&res, &variables)?;
                Ok(res)
            }
        } else {
            let operation_type = operation.meta.operation_type.clone();

            if operation_type == OperationType::Mutation {
                self.on_mutation::<Q, M>(&operation, &client);
            }

            let variables = operation.query.variables.clone();
            let res = self.next.run::<Q, M>(operation, client.clone()).await?;
            if operation_type == OperationType::Mutation {
                self.after_mutation::<Q, M>(&res, variables, &client);
            }
            Ok(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cache_exchange::NormalizedCacheExchange;
    use artemis::{
        client::ClientImpl, types::OperationOptions, Client, DebugInfo, Exchange, ExchangeFactory,
        ExchangeResult, GraphQLQuery, Operation, OperationResult, RequestPolicy, Response,
        ResultSource
    };
    use artemis_test::get_conference::{
        get_conference::{GetConferenceConference, ResponseData, Variables},
        GetConference
    };
    use std::{any::Any, sync::Arc};

    fn make_operation<Q: GraphQLQuery>(
        _query: Q,
        variables: Q::Variables
    ) -> Operation<Q::Variables> {
        let (query, meta) = Q::build_query(variables);
        Operation {
            query,
            meta,
            options: OperationOptions {
                url: "http://0.0.0.0".parse().unwrap(),
                request_policy: RequestPolicy::CacheFirst,
                extra_headers: None,
                extensions: None
            }
        }
    }

    struct DummyFetchExchange;
    impl<TNext: Exchange> ExchangeFactory<DummyFetchExchange, TNext> for DummyFetchExchange {
        fn build(self, _next: TNext) -> DummyFetchExchange {
            DummyFetchExchange
        }
    }

    #[async_trait]
    impl Exchange for DummyFetchExchange {
        async fn run<Q: GraphQLQuery, M: Exchange>(
            &self,
            operation: Operation<Q::Variables>,
            _client: Arc<ClientImpl<M>>
        ) -> ExchangeResult<Q::ResponseData> {
            let response_data = ResponseData {
                conference: Some(GetConferenceConference {
                    id: "0".to_string(),
                    name: "test".to_string(),
                    talks: None,
                    city: None
                })
            };
            let boxed: Box<dyn Any> = Box::new(response_data);

            let response_data = *boxed.downcast::<Q::ResponseData>().unwrap();
            Ok(OperationResult {
                meta: operation.meta,
                response: Response {
                    data: Some(response_data),
                    errors: None,
                    debug_info: Some(DebugInfo {
                        did_dedup: false,
                        source: ResultSource::Network
                    })
                }
            })
        }
    }

    #[tokio::test]
    async fn writes_queries_to_cache() {
        let client = Client::builder("http://0.0.0.0").build();
        let variables = Variables {
            id: "1".to_string()
        };

        let exchange = NormalizedCacheExchange::new().build(DummyFetchExchange);
        exchange
            .run::<GetConference, _>(
                make_operation(GetConference, variables.clone()),
                client.0.clone()
            )
            .await
            .unwrap();
        let result = exchange
            .run::<GetConference, _>(
                make_operation(GetConference, variables.clone()),
                client.0.clone()
            )
            .await;
        assert!(result.is_ok(), "Operation returned an error");
        let result = result.unwrap();

        assert_eq!(
            result.response.debug_info.unwrap().source,
            ResultSource::Cache,
            "Result didn't come from the cache"
        );
    }
}
