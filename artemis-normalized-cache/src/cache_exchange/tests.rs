use crate::{cache_exchange::NormalizedCacheExchange, NormalizedCacheExtension, QueryStore};
use artemis::{
    exchange::{
        Client, Exchange, ExchangeFactory, ExchangeResult, Operation, OperationMeta,
        OperationOptions, OperationResult
    },
    utils::progressive_hash,
    DebugInfo, Error, GraphQLQuery, RequestPolicy, Response, ResultSource
};
use artemis_test::{
    add_conference::{add_conference, add_conference::AddConferenceAddConference, AddConference},
    get_conference::{
        get_conference::{GetConferenceConference, ResponseData, Variables},
        GetConference
    },
    get_conferences::{
        get_conferences, get_conferences::GetConferencesConferences, GetConferences
    }
};
use racetrack::{track_with, Tracker};
use serde::de::DeserializeOwned;
use std::{any::Any, sync::Arc};

fn make_op_with_key<Q: GraphQLQuery>(
    _query: Q,
    variables: Q::Variables,
    key: u64
) -> Operation<Q::Variables> {
    let (query, meta) = Q::build_query(variables);
    Operation {
        key,
        query,
        meta: OperationMeta {
            query_key: key as u32,
            ..meta
        },
        options: OperationOptions {
            url: "http://0.0.0.0".parse().unwrap(),
            request_policy: RequestPolicy::CacheFirst,
            extra_headers: None,
            extensions: None
        }
    }
}

fn make_op<Q: GraphQLQuery>(_query: Q, variables: Q::Variables) -> Operation<Q::Variables> {
    let (query, meta) = Q::build_query(variables);
    Operation {
        key: progressive_hash(meta.query_key, &query.variables),
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

#[tokio::test]
async fn writes_queries_to_cache() {
    struct Fetch;
    #[async_trait]
    impl Exchange for Fetch {
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
        ) -> ExchangeResult<Q::ResponseData> {
            let response_data = ResponseData {
                conference: Some(GetConferenceConference {
                    id: "1".to_string(),
                    name: "test".to_string(),
                    talks: None,
                    city: None
                })
            };

            make_result::<Q>(operation, Box::new(response_data))
        }
    }

    let client = DummyClient {
        tracker: Tracker::new()
    };
    let variables = Variables {
        id: "1".to_string()
    };

    let operation = make_op_with_key(GetConference, variables.clone(), 1);

    let exchange = NormalizedCacheExchange::new().build(Fetch);
    exchange
        .run::<GetConference, _>(operation.clone(), client.clone())
        .await
        .unwrap();
    let result = exchange
        .run::<GetConference, _>(operation.clone(), client.clone())
        .await;
    assert!(result.is_ok(), "Operation returned an error");
    let result = result.unwrap();

    assert_eq!(
        result.response.debug_info.unwrap().source,
        ResultSource::Cache,
        "Result didn't come from the cache"
    );
}

lazy_static! {
    static ref TRACKER: Arc<Tracker> = Tracker::new();
}

fn make_result<Q: GraphQLQuery>(
    operation: Operation<Q::Variables>,
    data: Box<dyn Any>
) -> ExchangeResult<Q::ResponseData> {
    let data = *data.downcast::<Q::ResponseData>().unwrap();
    Ok(OperationResult {
        key: operation.key,
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

#[derive(Clone)]
struct DummyClient {
    tracker: Arc<Tracker>
}
#[track_with(tracker, namespace = "Client")]
impl Client for DummyClient {
    fn rerun_query(&self, _query_key: u64) {}

    fn push_result<R>(&self, _query_key: u64, _result: ExchangeResult<R>)
    where
        R: DeserializeOwned + Send + Sync + Clone + 'static
    {
    }
}

#[tokio::test]
async fn updates_related_queries() {
    let tracker = Tracker::new();

    struct Fetch {
        tracker: Arc<Tracker>
    }
    #[track_with(tracker)]
    #[async_trait]
    impl Exchange for Fetch {
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
        ) -> ExchangeResult<Q::ResponseData> {
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

            let query_key = operation.meta.query_key.clone();

            // This needs to be calculated at runtime because bincode is platform specific
            if query_key == 1 {
                make_result::<Q>(operation, Box::new(data_single))
            } else if query_key == 2 {
                make_result::<Q>(operation, Box::new(data_multi))
            } else {
                panic!("Exchange got called with invalid query {}", query_key)
            }
        }
    }

    let variables = Variables {
        id: "1".to_string()
    };
    let operation_single = make_op_with_key(GetConference, variables, 1);
    let operation_multiple = make_op_with_key(GetConferences, get_conferences::Variables, 2);

    let client = DummyClient {
        tracker: tracker.clone()
    };
    let dummy_exchange = Fetch {
        tracker: tracker.clone()
    };
    let exchange = NormalizedCacheExchange::new().build(dummy_exchange);

    let res = exchange
        .run::<GetConference, _>(operation_single, client.clone())
        .await;
    assert!(res.is_ok());
    tracker.assert_that("Fetch::run").was_called_once();

    let res = exchange
        .run::<GetConferences, _>(operation_multiple, client.clone())
        .await;
    assert!(res.is_ok());
    tracker.assert_that("Fetch::run").was_called_times(2);
    tracker
        .assert_that("Client::rerun_query")
        .was_called_once()
        .with((1u64));
}

#[tokio::test]
async fn does_nothing_when_no_related_queries_have_changed() {
    let tracker = Tracker::new();

    struct Fetch(Arc<Tracker>);
    #[track_with(0)]
    #[async_trait]
    impl Exchange for Fetch {
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
        ) -> ExchangeResult<Q::ResponseData> {
            let data_one = ResponseData {
                conference: Some(GetConferenceConference {
                    id: "1".to_string(),
                    name: "test".to_string(),
                    talks: None,
                    city: None
                })
            };
            let data_unrelated = ResponseData {
                conference: Some(GetConferenceConference {
                    id: "2".to_string(),
                    name: "test".to_string(),
                    talks: None,
                    city: None
                })
            };
            let query_key = &operation.meta.query_key;

            if query_key == &1 {
                make_result::<Q>(operation, Box::new(data_one))
            } else if query_key == &2 {
                make_result::<Q>(operation, Box::new(data_unrelated))
            } else {
                panic!("Received unexpected query with key {}", query_key);
            }
        }
    }

    let variables = Variables {
        id: "1".to_string()
    };
    let variables_unrelated = Variables {
        id: "2".to_string()
    };
    let operation_one = make_op_with_key(GetConference, variables, 1);
    let operation_unrelated = make_op_with_key(GetConference, variables_unrelated, 2);

    let client = DummyClient {
        tracker: tracker.clone()
    };
    let exchange = NormalizedCacheExchange::new().build(Fetch(tracker.clone()));

    let res = exchange
        .run::<GetConference, _>(operation_one.clone(), client.clone())
        .await;
    assert!(res.is_ok());

    let res = exchange
        .run::<GetConference, _>(operation_unrelated.clone(), client.clone())
        .await;
    assert!(res.is_ok());

    tracker.assert_that("Client::rerun_query").wasnt_called();
}

#[tokio::test]
async fn writes_optimistic_mutations_to_the_cache() {
    let tracker = Tracker::new();

    struct Fetch(Arc<Tracker>);

    #[track_with(0)]
    #[async_trait]
    impl Exchange for Fetch {
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
        ) -> ExchangeResult<Q::ResponseData> {
            let data_one = ResponseData {
                conference: Some(GetConferenceConference {
                    id: "1".to_string(),
                    name: "test".to_string(),
                    talks: None,
                    city: None
                })
            };
            let data_mutation = add_conference::ResponseData {
                add_conference: Some(AddConferenceAddConference {
                    id: "1".to_string(),
                    name: "test3".to_string(),
                    talks: None,
                    city: None
                })
            };

            let query_key = &operation.meta.query_key;

            if query_key == &1 {
                make_result::<Q>(operation, Box::new(data_one))
            } else if query_key == &2 {
                make_result::<Q>(operation, Box::new(data_mutation))
            } else {
                panic!("Received unexpected query with key {}", query_key);
            }
        }
    }

    #[track_with(tracker)]
    let optimistic = || {
        Some(add_conference::ResponseData {
            add_conference: Some(AddConferenceAddConference {
                id: "1".to_string(),
                name: "test3".to_string(),
                talks: None,
                city: None
            })
        })
    };

    let client = DummyClient {
        tracker: tracker.clone()
    };
    let exchange = NormalizedCacheExchange::new().build(Fetch(tracker.clone()));

    let op_one = make_op_with_key(
        GetConference,
        Variables {
            id: "1".to_string()
        },
        1
    );
    let op_mut = {
        let variables = add_conference::Variables {
            name: "test3".to_string(),
            city: None
        };
        let (query, meta) = <AddConference as GraphQLQuery>::build_query(variables);
        let extension =
            NormalizedCacheExtension::new().optimistic_result::<AddConference, _>(optimistic);
        Operation {
            key: 2,
            query,
            meta: OperationMeta {
                query_key: 2,
                ..meta
            },
            options: OperationOptions {
                url: "http://0.0.0.0".parse().unwrap(),
                request_policy: RequestPolicy::CacheFirst,
                extra_headers: None,
                extensions: Some(artemis::ext![extension])
            }
        }
    };

    let res = exchange
        .run::<GetConference, _>(op_one.clone(), client.clone())
        .await;
    assert!(res.is_ok());

    let res = exchange
        .run::<AddConference, _>(op_mut.clone(), client.clone())
        .await;
    assert!(res.is_ok());
    tracker.assert_that("optimistic").was_called_once();
    tracker
        .assert_that("Client::rerun_query")
        .was_called_times(2)
        .with((1u64))
        .not_with((2u64));
    tracker.assert_that("Fetch::run").was_called_times(2);
}

#[tokio::test]
async fn correctly_clears_on_error() {
    TRACKER.clear();
    let tracker = Tracker::new();

    #[track_with(TRACKER, namespace = "clear_on_error")]
    fn optimistic() -> Option<add_conference::ResponseData> {
        Some(add_conference::ResponseData {
            add_conference: Some(AddConferenceAddConference {
                id: "asd".to_string(),
                name: "test3".to_string(),
                talks: None,
                city: None
            })
        })
    }

    #[track_with(TRACKER, namespace = "clear_on_error")]
    fn update(
        data: &Option<add_conference::ResponseData>,
        store: QueryStore,
        dependencies: &mut Vec<String>
    ) {
        println!("Update Data: {:?}", data);
        if let Some(conference) = data.as_ref().and_then(|data| data.add_conference.as_ref()) {
            store.update_query(
                GetConferences,
                get_conferences::Variables,
                |current_data| {
                    let result = if let Some(current_data) = current_data {
                        Some(get_conferences::ResponseData {
                            conferences: current_data.conferences.map(|vec| {
                                let mut vec: Vec<_> = vec.iter().cloned().collect();
                                vec.push(GetConferencesConferences {
                                    id: conference.id.clone(),
                                    name: conference.name.clone()
                                });
                                vec
                            })
                        })
                    } else {
                        None
                    };
                    result
                },
                dependencies
            );
        } else {
            store.update_query(
                GetConferences,
                get_conferences::Variables,
                |current_data| {
                    if current_data.is_some() {
                        Some(get_conferences::ResponseData {
                            conferences: Some(Vec::new())
                        })
                    } else {
                        None
                    }
                },
                dependencies
            );
        }
    }

    let operation_one = make_op(GetConferences, get_conferences::Variables);
    let variables_mutation = add_conference::Variables {
        name: "test2".to_string(),
        city: None
    };
    let operation_mutation = {
        let (query, meta) = <AddConference as GraphQLQuery>::build_query(variables_mutation);
        let extension = NormalizedCacheExtension::new()
            .optimistic_result::<AddConference, _>(optimistic)
            .update::<AddConference, _>(update);
        Operation {
            key: 2,
            query,
            meta: OperationMeta {
                query_key: 2,
                ..meta
            },
            options: OperationOptions {
                url: "http://0.0.0.0".parse().unwrap(),
                request_policy: RequestPolicy::CacheFirst,
                extra_headers: None,
                extensions: Some(artemis::ext![extension])
            }
        }
    };

    struct Fetch(Arc<Tracker>);

    #[track_with(0)]
    #[async_trait]
    impl Exchange for Fetch {
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
        ) -> ExchangeResult<Q::ResponseData> {
            let data_one = get_conferences::ResponseData {
                conferences: Some(vec![GetConferencesConferences {
                    id: "1".to_string(),
                    name: "test".to_string()
                }])
            };

            let query_key = &operation.meta.query_key;

            if query_key == &2 {
                Ok(OperationResult {
                    key: operation.key,
                    meta: operation.meta,
                    response: Response {
                        data: None,
                        errors: Some(vec![Error {
                            path: None,
                            extensions: None,
                            locations: None,
                            message: "Test error".to_string()
                        }]),
                        debug_info: None
                    }
                })
            } else {
                make_result::<Q>(operation, Box::new(data_one))
            }
        }
    }

    let client = DummyClient {
        tracker: tracker.clone()
    };
    let exchange = NormalizedCacheExchange::new().build(Fetch(tracker.clone()));

    let res = exchange
        .run::<GetConferences, _>(operation_one.clone(), client.clone())
        .await;
    assert!(res.is_ok());
    let res = exchange
        .run::<AddConference, _>(operation_mutation.clone(), client.clone())
        .await;
    assert!(res.is_ok());
    TRACKER
        .assert_that("clear_on_error::optimistic")
        .was_called_once();
    TRACKER
        .assert_that("clear_on_error::update")
        .was_called_times(2);
    tracker
        .assert_that("Client::rerun_query")
        .was_called_times(2);
}

#[tokio::test]
async fn follows_optimistic_on_initial_write() {
    let tracker = Tracker::new();

    let client = DummyClient {
        tracker: tracker.clone()
    };
    let mut op_one = make_op_with_key(
        GetConference,
        Variables {
            id: "1".to_string()
        },
        1
    );

    struct Fetch(Arc<Tracker>);

    #[track_with(0)]
    #[async_trait]
    impl Exchange for Fetch {
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
        ) -> ExchangeResult<Q::ResponseData> {
            let data_one = ResponseData {
                conference: Some(GetConferenceConference {
                    id: "1".to_string(),
                    name: "test".to_string(),
                    talks: None,
                    city: None
                })
            };

            if operation.key == 1 {
                make_result::<Q>(operation, Box::new(data_one))
            } else {
                unreachable!()
            }
        }
    }

    let optimistic_data = ResponseData {
        conference: Some(GetConferenceConference {
            id: "1".to_string(),
            name: "other_test_name".to_string(),
            talks: None,
            city: None
        })
    };

    #[track_with(tracker)]
    let optimistic = || {
        Some(ResponseData {
            conference: Some(GetConferenceConference {
                id: "1".to_string(),
                name: "other_test_name".to_string(),
                talks: None,
                city: None
            })
        })
    };

    let exchange = NormalizedCacheExchange::new().build(Fetch(tracker.clone()));
    let extension =
        NormalizedCacheExtension::new().optimistic_result::<GetConference, _>(optimistic);
    op_one.options.extensions = Some(artemis::ext![extension]);

    let res = exchange
        .run::<GetConference, _>(op_one.clone(), client)
        .await;
    assert!(res.is_ok());
    let data: ResponseData = res.unwrap().response.data.unwrap();
    assert_eq!(&data.conference.unwrap().name, "test");
    tracker.assert_that("optimistic").was_called_once();

    let push_data: (u64, ExchangeResult<ResponseData>) = (
        1u64,
        Ok(OperationResult {
            key: 1,
            meta: op_one.meta,
            response: Response {
                data: Some(optimistic_data),
                errors: None,
                debug_info: None
            }
        })
    );

    tracker
        .assert_that("Client::push_result")
        .was_called_once()
        .with(push_data);
}
