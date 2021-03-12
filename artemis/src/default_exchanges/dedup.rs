use crate::{
    exchange::Client,
    types::{ExchangeResult, Operation, OperationResult},
    Exchange, ExchangeFactory, GraphQLQuery, OperationType, QueryError
};
use futures::channel::{oneshot, oneshot::Sender};
use std::{
    any::Any,
    collections::HashMap,
    error::Error,
    fmt,
    sync::{Arc, Mutex}
};

type InFlightCache = Arc<Mutex<HashMap<u64, Vec<Sender<Result<Box<dyn Any + Send>, QueryError>>>>>>;

/// The default deduplication exchange.
///
/// This will keep track of in-flight queries and catch any identical queries before they execute,
/// instead waiting for the result from the in-flight query
pub struct DedupExchange;
pub struct DedupExchangeImpl<TNext: Exchange> {
    next: TNext,
    in_flight_operations: InFlightCache
}

impl<TNext: Exchange> ExchangeFactory<TNext> for DedupExchange {
    type Output = DedupExchangeImpl<TNext>;

    fn build(self, next: TNext) -> Self::Output {
        DedupExchangeImpl {
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

fn should_skip<Q: GraphQLQuery>(operation: &Operation<Q::Variables>) -> bool {
    let op_type = &operation.meta.operation_type;
    op_type != &OperationType::Query && op_type != &OperationType::Mutation
}

fn make_deduped_result<Q: GraphQLQuery>(
    res: &ExchangeResult<Q::ResponseData>
) -> Result<Box<dyn Any + Send>, QueryError> {
    match res {
        Ok(ref res) => {
            let mut res = res.clone();
            if let Some(ref mut debug_info) = res.response.debug_info {
                debug_info.did_dedup = true;
            }
            Ok(Box::new(res))
        }
        Err(e) => Err(e.clone())
    }
}

impl<TNext: Exchange> DedupExchangeImpl<TNext> {
    fn notify_listeners<Q: GraphQLQuery>(&self, key: u64, res: &ExchangeResult<Q::ResponseData>) {
        let mut cache = self.in_flight_operations.lock().unwrap();
        let to_be_notified = cache.remove(&key).unwrap();
        for sender in to_be_notified {
            let res = make_deduped_result::<Q>(res);
            sender.send(res).unwrap();
        }
    }
}

#[async_trait]
impl<TNext: Exchange> Exchange for DedupExchangeImpl<TNext> {
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        operation: Operation<Q::Variables>,
        _client: C
    ) -> ExchangeResult<Q::ResponseData> {
        if should_skip::<Q>(&operation) {
            return self.next.run::<Q, _>(operation, _client).await;
        }

        let key = operation.key;
        let rcv = {
            let mut cache = self.in_flight_operations.lock().unwrap();
            if let Some(listeners) = cache.get_mut(&key) {
                let (sender, receiver) = oneshot::channel();
                listeners.push(sender);
                Some(receiver)
            } else {
                cache.insert(key, Vec::new());
                None
            }
        };

        if let Some(rcv) = rcv {
            let res: Box<dyn Any> = rcv.await.unwrap()?;
            let res: OperationResult<Q::ResponseData> = *res.downcast().unwrap();
            Ok(res)
        } else {
            let res = self.next.run::<Q, _>(operation, _client).await;
            self.notify_listeners::<Q>(key, &res);
            res
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod test {
    use super::DedupExchangeImpl;
    use crate::{
        default_exchanges::DedupExchange,
        exchange::Client,
        types::{Operation, OperationOptions, OperationResult},
        ClientBuilder, DebugInfo, Exchange, ExchangeFactory, ExchangeResult, FieldSelector,
        GraphQLQuery, OperationMeta, OperationType, QueryBody, QueryInfo, RequestPolicy, Response,
        ResultSource
    };
    use artemis_test::get_conference::{
        get_conference::{ResponseData, Variables, OPERATION_NAME, QUERY},
        GetConference
    };
    use lazy_static::lazy_static;
    use std::time::Duration;
    use tokio::time::sleep;

    lazy_static! {
        static ref VARIABLES: Variables = Variables {
            id: "1".to_string()
        };
        static ref EXCHANGE: DedupExchangeImpl<FakeFetchExchange> =
            DedupExchange.build(FakeFetchExchange);
    }

    fn url() -> String {
        "http://localhost:8080/graphql".to_string()
    }

    struct FakeFetchExchange;

    impl<TNext: Exchange> ExchangeFactory<TNext> for FakeFetchExchange {
        type Output = FakeFetchExchange;

        fn build(self, _next: TNext) -> FakeFetchExchange {
            Self
        }
    }

    #[async_trait]
    impl Exchange for FakeFetchExchange {
        async fn run<Q: GraphQLQuery, C: Client>(
            &self,
            operation: Operation<Q::Variables>,
            _client: C
        ) -> ExchangeResult<Q::ResponseData> {
            sleep(Duration::from_millis(10)).await;
            let res = OperationResult {
                key: operation.key,
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
            key: meta.query_key as u64,
            meta,
            query,
            options: OperationOptions {
                request_policy: RequestPolicy::NetworkOnly,
                extra_headers: None,
                url: url(),
                extensions: None
            }
        }
    }

    fn build_query(variables: Variables) -> (QueryBody<Variables>, OperationMeta) {
        let meta = OperationMeta {
            query_key: 13543040u32,
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

    impl GraphQLQuery for GetConference {
        type Variables = Variables;
        type ResponseData = ResponseData;

        fn build_query(_variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta) {
            unimplemented!()
        }
    }

    impl QueryInfo<Variables> for ResponseData {
        fn selection(_variables: &Variables) -> Vec<FieldSelector> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_dedup() {
        let (query, meta) = build_query(VARIABLES.clone());

        let client = ClientBuilder::new("http://localhost:4000/graphql").build();

        let fut1 = EXCHANGE.run::<GetConference, _>(
            make_operation(query.clone(), meta.clone()),
            client.0.clone()
        );
        let fut2 = EXCHANGE.run::<GetConference, _>(
            make_operation(query.clone(), meta.clone()),
            client.0.clone()
        );
        let join = tokio::spawn(async { fut1.await.unwrap() });
        let res2 = fut2.await.unwrap();
        let res1 = join.await.unwrap();

        // The order can vary depending on the executor state, so XOR them
        let did_1_dedup = res1.response.debug_info.unwrap().did_dedup;
        let did_2_dedup = res2.response.debug_info.unwrap().did_dedup;
        let did_one_dedup = did_1_dedup ^ did_2_dedup;

        assert_eq!(did_one_dedup, true);
    }
}
