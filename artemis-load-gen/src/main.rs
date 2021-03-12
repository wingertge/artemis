#[macro_use]
extern crate async_trait;

use crate::queries::get_conference::{get_conference::Variables, GetConference};
use artemis::{
    exchange::{Client, Exchange, ExchangeFactory, ExchangeResult, Operation, OperationResult},
    ClientBuilder, GraphQLQuery, Response
};
use artemis_normalized_cache::NormalizedCacheExchange;
use rand::Rng;
use std::{
    any::Any,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc
    },
    thread,
    time::Duration
};
use tokio::runtime::Runtime;

mod queries;

pub(crate) type Long = String;

struct DummyFetchExchange;

impl<TNext: Exchange> ExchangeFactory<TNext> for DummyFetchExchange {
    type Output = DummyFetchExchange;

    fn build(self, _next: TNext) -> DummyFetchExchange {
        Self
    }
}

#[async_trait]
impl Exchange for DummyFetchExchange {
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        operation: Operation<Q::Variables>,
        _client: C
    ) -> ExchangeResult<Q::ResponseData> {
        use crate::queries::get_conference::get_conference::{
            GetConferenceConference, ResponseData
        };

        let delay = Duration::from_millis(50);
        tokio::time::sleep(delay).await;
        let variables: Box<dyn Any> = Box::new(operation.query.variables);
        let variables = (&variables).downcast_ref::<Variables>().unwrap();
        let data = ResponseData {
            conference: Some(GetConferenceConference {
                id: variables.id.clone(),
                city: Some("Test City".to_string()),
                name: "Test Conference".to_string(),
                talks: Some(Vec::new())
            })
        };

        let data: Box<dyn Any> = Box::new(data);
        let data = *data.downcast::<Q::ResponseData>().unwrap();

        let result = OperationResult {
            key: operation.key,
            meta: operation.meta,
            response: Response {
                data: Some(data),
                debug_info: None,
                errors: None
            }
        };

        Ok(result)
    }
}

#[cfg(target_os = "linux")]
fn begin() {
    coz::begin!("query");
}

#[cfg(not(target_os = "linux"))]
fn begin() {}

#[cfg(target_os = "linux")]
fn end() {
    coz::end!("query")
}

#[cfg(not(target_os = "linux"))]
fn end() {}

#[allow(clippy::infinite_iter)]
fn main() {
    let url = "http://localhost:8080/graphql";
    let builder = ClientBuilder::new(url)
        .with_exchange(DummyFetchExchange)
        .with_exchange(NormalizedCacheExchange::new());
    //.with_exchange(DedupExchange);
    let client = Arc::new(builder.build());

    println!("Started");

    let n = 25;
    let variable_set: Vec<Variables> = (0..n).map(|i| Variables { id: i.to_string() }).collect();

    let thread_count = 1;

    let query_count = Arc::new(AtomicU32::new(0));

    for _ in 0..thread_count {
        let client = client.clone();
        let variable_set = variable_set.clone();
        let query_count = query_count.clone();
        let runtime = Runtime::new().unwrap();
        thread::spawn(move || loop {
            let futs = (0..100).map(|_| {
                let var_id = rand::thread_rng().gen_range(0, n);
                let variables = variable_set.get(var_id).cloned().unwrap();
                let client = client.clone();
                async move {
                    begin();
                    client.query(GetConference, variables).await.unwrap();
                    end();
                }
            });

            runtime.block_on(futures::future::join_all(futs));
            query_count.fetch_add(100, Ordering::SeqCst);
        });
    }

    thread::spawn(move || {
        let mut seconds = 0;
        loop {
            println!(
                "Query Count: {} at {}s",
                query_count.load(Ordering::SeqCst),
                seconds
            );
            thread::sleep(Duration::from_secs(1));
            seconds += 1;
        }
    })
    .join()
    .unwrap();
}
