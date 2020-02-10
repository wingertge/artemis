use crate::{types::{ExchangeResult, Operation, OperationResult}, Exchange, ExchangeFactory, Response, OperationType};
use futures::channel::oneshot::{self, Sender};
use serde::Serialize;
use std::{collections::HashMap, sync::{Arc, Mutex}, fmt};
use std::error::Error;

type InFlightCache = Arc<Mutex<HashMap<u32, Vec<Sender<Result<OperationResult, DedupError>>>>>>;

pub struct DedupExchange<TNext: Exchange> {
    next: TNext,
    in_flight_operations: InFlightCache
}

impl<TNext: Exchange> ExchangeFactory<DedupExchange<TNext>, TNext> for DedupExchange<TNext> {
    fn build(next: TNext) -> DedupExchange<TNext> {
        DedupExchange {
            next,
            in_flight_operations: InFlightCache::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct DedupError;
impl Error for DedupError {}
impl fmt::Display for DedupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "") //TODO: This isn't ideal
    }
}

fn should_skip<V: Serialize + Send + Sync>(operation: &Operation<V>) -> bool {
    let op_type = &operation.meta.operation_type;
    op_type != &OperationType::Query && op_type != &OperationType::Mutation
}

#[async_trait]
impl<TNext: Exchange> Exchange for DedupExchange<TNext> {
    async fn run<V: Serialize + Send + Sync>(&self, operation: Operation<V>) -> ExchangeResult {
        if should_skip(&operation) {
            return self.next.run(operation).await;
        }

        let key = operation.meta.key.clone();
        let contains = {
            let cache = self.in_flight_operations.lock().unwrap();
            cache.contains_key(&key)
        };

        if contains {
            let (sender, receiver) = oneshot::channel();
            let retry = {
                let mut cache = self.in_flight_operations.lock().unwrap();
                let in_flight = cache.get_mut(&key);
                if let Some(in_flight) = in_flight {
                    in_flight.push(sender);
                    false
                } else {
                    true // The response arrived while the cache was unlocked, retry
                }
            };
            if retry {
                return self.run(operation).await;
            }
            Ok(receiver.await.unwrap()?)
        } else {
            {
                let mut cache = self.in_flight_operations.lock().unwrap();
                cache.insert(key.clone(), Vec::new());
            }

            let res = self.next.run(operation).await;
            let to_be_notified = {
                let mut cache = self.in_flight_operations.lock().unwrap();
                cache.remove(&key).unwrap()
            };

            for sender in to_be_notified {
                let mut res = match res {
                    Ok(ref res) => Ok(res.clone()),
                    Err(_) => Err(DedupError)
                };

                if let Ok(OperationResult {
                              response:
                              Response {
                                  debug_info: Some(ref mut debug_info),
                                  ..
                              },
                              ..
                          }) = res
                {
                    debug_info.did_dedup = true;
                }
                sender.send(res).unwrap();
            }
            res
        }
    }
}

#[cfg(test)]
mod test {
    use artemis_test::get_conference::{get_conference::{Variables, QUERY, OPERATION_NAME}};
    use crate::{ExchangeFactory, Exchange, OperationMeta, QueryBody, Response, DebugInfo, ResultSource, RequestPolicy, Url, OperationType};
    use std::error::Error;
    use crate::types::{OperationResult, Operation};
    use serde::Serialize;
    use tokio::time::delay_for;
    use std::time::Duration;
    use crate::exchanges::DedupExchange;
    use lazy_static::lazy_static;

    lazy_static! {
        static ref VARIABLES: Variables = Variables { id: "1".to_string() };
        static ref EXCHANGE: DedupExchange<FakeFetchExchange> = DedupExchange::build(FakeFetchExchange);
    }

    fn url() -> Url {
        "http://localhost:8080/graphql".parse().unwrap()
    }

    struct FakeFetchExchange;

    impl <TNext: Exchange> ExchangeFactory<FakeFetchExchange, TNext> for FakeFetchExchange {
        fn build(_next: TNext) -> FakeFetchExchange {
            Self
        }
    }

    #[async_trait]
    impl Exchange for FakeFetchExchange {
        async fn run<V: Serialize + Send + Sync>(&self, operation: Operation<V>) -> Result<OperationResult, Box<dyn Error>> {
            delay_for(Duration::from_millis(10)).await;
            let res = OperationResult {
                meta: operation.meta,
                response: Response {
                    debug_info: Some(DebugInfo {
                        source: ResultSource::Network,
                        did_dedup: false
                    }),
                    data: None,
                    errors: None
                }
            };
            Ok(res)
        }
    }

    fn make_operation(query: QueryBody<Variables>, meta: OperationMeta) -> Operation<Variables> {
        Operation {
            meta,
            query,
            request_policy: RequestPolicy::NetworkOnly,
            extra_headers: None,
            url: url()
        }
    }

    fn build_query(
        variables: Variables
    ) -> (
        QueryBody<Variables>,
        OperationMeta
    ) {
        let meta = OperationMeta {
            key: 1354603040u32,
            operation_type: OperationType::Query,
            involved_types: vec!["Conference", "Person", "Talk"]
        };
        let body = QueryBody {
            variables,
            query: QUERY,
            operation_name: OPERATION_NAME
        };
        (body, meta)
    }

    #[tokio::test]
    async fn test_dedup() {
        let (query, meta) = build_query(VARIABLES.clone());

        let fut1 = EXCHANGE.run(make_operation(query.clone(), meta.clone()));
        let fut2 = EXCHANGE.run(make_operation(query.clone(), meta.clone()));
        let join = tokio::spawn(async {
            fut1.await.unwrap()
        });
        let res2 = fut2.await.unwrap();
        let res1 = join.await.unwrap();

        // The order can vary depending on the executor state, so XOR them
        let did_1_dedup = res1.response.debug_info.unwrap().did_dedup;
        let did_2_dedup = res2.response.debug_info.unwrap().did_dedup;
        let did_one_dedup = did_1_dedup ^ did_2_dedup;

        assert_eq!(did_one_dedup, true);
    }
}
