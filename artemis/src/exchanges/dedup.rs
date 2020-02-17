use crate::{types::{ExchangeResult, Operation, OperationResult}, Exchange, ExchangeFactory, OperationType, GraphQLQuery};
use futures::channel::oneshot::{self, Sender};
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    sync::{Arc, Mutex}
};
use std::any::Any;

type InFlightCache = Arc<Mutex<HashMap<u64, Vec<Sender<Result<Box<dyn Any + Send>, DedupError>>>>>>;

pub struct DedupExchange; // Factory
pub struct DedupExchangeImpl<TNext: Exchange> {
    next: TNext,
    in_flight_operations: InFlightCache
}

impl<TNext: Exchange> ExchangeFactory<DedupExchangeImpl<TNext>, TNext> for DedupExchange {
    fn build(self, next: TNext) -> DedupExchangeImpl<TNext> {
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

fn make_deduped_result<Q: GraphQLQuery>(res: &ExchangeResult<Q::ResponseData>) -> Result<Box<dyn Any + Send>, DedupError> {
    match res {
        Ok(ref res) => {
            let mut res = res.clone();
            if let Some(ref mut debug_info) = res.response.debug_info {
                debug_info.did_dedup = true;
            }
            Ok(Box::new(res))
        }
        Err(_) => Err(DedupError)
    }
}

impl<TNext: Exchange> DedupExchangeImpl<TNext> {
    fn notify_listeners<Q: GraphQLQuery>(&self, key: &u64, res: &ExchangeResult<Q::ResponseData>) {
        let mut cache = self.in_flight_operations.lock().unwrap();
        let to_be_notified = cache.remove(key).unwrap();
        for sender in to_be_notified {
            let res = make_deduped_result::<Q>(res);
            sender.send(res).unwrap();
        }
    }
}

#[async_trait]
impl<TNext: Exchange> Exchange for DedupExchangeImpl<TNext> {
    async fn run<Q: GraphQLQuery>(&self, operation: Operation<Q::Variables>) -> ExchangeResult<Q::ResponseData> {
        if should_skip::<Q>(&operation) {
            return self.next.run::<Q>(operation).await;
        }

        let key = operation.meta.key.clone();
        let rcv = {
            let mut cache = self.in_flight_operations.lock().unwrap();
            if let Some(listeners) = cache.get_mut(&key) {
                let (sender, receiver) = oneshot::channel();
                listeners.push(sender);
                Some(receiver)
            } else {
                cache.insert(key.clone(), Vec::new());
                None
            }
        };

        if let Some(rcv) = rcv {
            let res: Box<dyn Any> = rcv.await.unwrap()?;
            let res: OperationResult<Q::ResponseData> = *res.downcast().unwrap();
            Ok(res)
        } else {
            let res = self.next.run::<Q>(operation).await;
            self.notify_listeners::<Q>(&key, &res);
            res
        }
    }
}

#[cfg(test)]
mod test {
    use super::DedupExchangeImpl;
    use crate::{exchanges::DedupExchange, types::{Operation, OperationResult}, DebugInfo, Exchange, ExchangeFactory, OperationMeta, OperationType, QueryBody, RequestPolicy, Response, ResultSource, Url, GraphQLQuery, QueryInfo, FieldSelector};
    use artemis_test::get_conference::get_conference::{Variables, OPERATION_NAME, QUERY, ResponseData};
    use lazy_static::lazy_static;
    use std::{error::Error, time::Duration};
    use tokio::time::delay_for;
    use artemis_test::get_conference::GetConference;

    lazy_static! {
        static ref VARIABLES: Variables = Variables {
            id: "1".to_string()
        };
        static ref EXCHANGE: DedupExchangeImpl<FakeFetchExchange> =
            DedupExchange.build(FakeFetchExchange);
    }

    fn url() -> Url {
        "http://localhost:8080/graphql".parse().unwrap()
    }

    struct FakeFetchExchange;

    impl<TNext: Exchange> ExchangeFactory<FakeFetchExchange, TNext> for FakeFetchExchange {
        fn build(self, _next: TNext) -> FakeFetchExchange {
            Self
        }
    }

    #[async_trait]
    impl Exchange for FakeFetchExchange {
        async fn run<Q: GraphQLQuery>(
            &self,
            operation: Operation<Q::Variables>
        ) -> Result<OperationResult<Q::ResponseData>, Box<dyn Error>> {
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

    fn build_query(variables: Variables) -> (QueryBody<Variables>, OperationMeta) {
        let meta = OperationMeta {
            key: 1354603040u64,
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
        fn typename(&self) -> &'static str {
            unimplemented!()
        }

        fn selection(_variables: &Variables) -> Vec<FieldSelector> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_dedup() {
        let (query, meta) = build_query(VARIABLES.clone());

        let fut1 = EXCHANGE.run::<GetConference>(make_operation(query.clone(), meta.clone()));
        let fut2 = EXCHANGE.run::<GetConference>(make_operation(query.clone(), meta.clone()));
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
