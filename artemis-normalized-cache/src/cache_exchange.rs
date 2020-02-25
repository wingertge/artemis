use crate::{
    store::Store,
    types::{NormalizedCacheExtension, NormalizedCacheOptions}
};
use artemis::{
    exchanges::Client, DebugInfo, Exchange, ExchangeFactory, ExchangeResult, GraphQLQuery,
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
    fn after_query<Q: GraphQLQuery, C: Client>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: &Q::Variables,
        client: &C
    ) -> Result<(), QueryError> {
        self.store
            .write_query::<Q, _>(result, variables, false, client)
    }

    fn after_mutation<Q: GraphQLQuery, C: Client>(
        &self,
        result: &OperationResult<Q::ResponseData>,
        variables: Q::Variables,
        client: &C
    ) {
        self.store
            .invalidate_query::<Q, _>(result, &variables, client, false);
        self.store
            .write_query::<Q, _>(result, &variables, false, client)
            .unwrap();
    }

    fn on_mutation<Q: GraphQLQuery, C: Client>(
        &self,
        operation: &Operation<Q::Variables>,
        client: &C
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
                    .invalidate_query::<Q, _>(&result, variables, client, true);
                self.store
                    .write_query::<Q, _>(&result, variables, true, client)
                    .unwrap();
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
                let res = self.next.run::<Q, _>(operation, client.clone()).await?;
                self.after_query::<Q, _>(&res, &variables, &client)?;
                Ok(res)
            }
        } else {
            let operation_type = operation.meta.operation_type.clone();

            if operation_type == OperationType::Mutation {
                self.on_mutation::<Q, _>(&operation, &client);
            }

            let variables = operation.query.variables.clone();
            let res = self.next.run::<Q, _>(operation, client.clone()).await?;
            if operation_type == OperationType::Mutation {
                self.after_mutation::<Q, _>(&res, variables, &client);
            }
            Ok(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cache_exchange::NormalizedCacheExchange;
    use artemis::{exchanges::Client, types::OperationOptions, ClientBuilder, DebugInfo, Exchange, ExchangeFactory, ExchangeResult, GraphQLQuery, Operation, OperationResult, RequestPolicy, Response, ResultSource, progressive_hash};
    use artemis_test::{
        get_conference::{
            get_conference::{GetConferenceConference, ResponseData, Variables},
            GetConference
        },
        get_conferences::{
            get_conferences, get_conferences::GetConferencesConferences, GetConferences
        },
        Counter, SyncCounter
    };
    use std::any::Any;

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
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
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
        let client = ClientBuilder::new("http://0.0.0.0").build();
        let variables = Variables {
            id: "1".to_string()
        };

        let operation = make_operation(GetConference, variables.clone());

        let exchange = NormalizedCacheExchange::new().build(DummyFetchExchange);
        exchange
            .run::<GetConference, _>(operation.clone(), client.0.clone())
            .await
            .unwrap();
        let result = exchange
            .run::<GetConference, _>(operation.clone(), client.0.clone())
            .await;
        assert!(result.is_ok(), "Operation returned an error");
        let result = result.unwrap();

        assert_eq!(
            result.response.debug_info.unwrap().source,
            ResultSource::Cache,
            "Result didn't come from the cache"
        );
    }

    fn make_result<Q: GraphQLQuery>(
        operation: Operation<Q::Variables>,
        data: Box<dyn Any>
    ) -> ExchangeResult<Q::ResponseData> {
        let data = *data.downcast::<Q::ResponseData>().unwrap();
        Ok(OperationResult {
            meta: operation.meta,
            response: Response {
                debug_info: Some(DebugInfo {
                    source: ResultSource::Network,
                    did_dedup: false
                }),
                errors: None,
                data: Some(data)
            }
        })
    }

    #[tokio::test]
    async fn updates_related_queries() {
        #[derive(Clone)]
        struct DummyExchange {
            called: SyncCounter
        }
        #[async_trait]
        impl Exchange for DummyExchange {
            async fn run<Q: GraphQLQuery, C: Client>(
                &self,
                operation: Operation<<Q as GraphQLQuery>::Variables>,
                _client: C
            ) -> ExchangeResult<<Q as GraphQLQuery>::ResponseData> {
                Counter::inc_sync(&self.called);

                let key_single = 8181565099941403168u64;
                let key_multiple = 11949895552938567266u64;

                let data_single = ResponseData {
                    conference: Some(GetConferenceConference {
                        id: "1".to_string(),
                        name: "test".to_string(),
                        talks: None,
                        city: None
                    })
                };
                let data_multi = get_conferences::ResponseData {
                    conferences: Some(vec![GetConferencesConferences {
                        id: "1".to_string(),
                        name: "test".to_string()
                    }])
                };

                let query_key = operation.meta.key.clone();

                // This needs to be calculated at runtime because bincode is platform specific
                if query_key == progressive_hash(key_single, &operation.query.variables) {
                    make_result::<Q>(operation, Box::new(data_single))
                } else if query_key == progressive_hash(key_multiple, &operation.query.variables) {
                    make_result::<Q>(operation, Box::new(data_multi))
                } else {
                    panic!("Exchange got called with invalid query {}", query_key)
                }
            }
        }
        impl DummyExchange {
            fn was_called(&self) -> u32 {
                Counter::get_sync(&self.called)
            }
        }

        #[derive(Clone)]
        struct DummyClient {
            called: SyncCounter
        }
        impl Client for DummyClient {
            fn rerun_query(&self, _query_key: u64) {
                Counter::inc_sync(&self.called);
            }
        }
        impl DummyClient {
            fn was_called(&self) -> u32 {
                Counter::get_sync(&self.called)
            }
        }

        let variables = Variables {
            id: "1".to_string()
        };
        let operation_single = make_operation(GetConference, variables);
        let operation_multiple = make_operation(
            GetConferences,
            get_conferences::Variables
        );

        let client = DummyClient {
            called: Counter::sync()
        };
        let dummy_exchange = DummyExchange {
            called: Counter::sync()
        };
        let exchange = NormalizedCacheExchange::new().build(dummy_exchange.clone());

        let res = exchange
            .run::<GetConference, _>(operation_single, client.clone())
            .await;
        assert!(res.is_ok());
        assert_eq!(dummy_exchange.was_called(), 1, "Exchange was called more than once");

        let res = exchange
            .run::<GetConferences, _>(operation_multiple, client.clone())
            .await;
        assert!(res.is_ok());
        assert_eq!(dummy_exchange.was_called(), 2, "Exchange was called more than twice");
        assert_eq!(client.was_called(), 1, "Rerun queries was called more than once");
    }
}
