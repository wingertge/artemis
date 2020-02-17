#[macro_use]
extern crate async_trait;

use crate::queries::get_conference::{get_conference::Variables, GetConference};
use artemis::{exchanges::{CacheExchange, DedupExchange}, ClientBuilder, Exchange, ExchangeFactory, Operation, OperationResult, Response, GraphQLQuery};
use rand::Rng;
use rayon::{iter, iter::ParallelIterator};
use std::{error::Error, sync::Arc, time::Duration};
use std::any::Any;

mod queries;

pub(crate) type Long = String;

struct DummyFetchExchange;

impl<TNext: Exchange> ExchangeFactory<DummyFetchExchange, TNext> for DummyFetchExchange {
    fn build(self, _next: TNext) -> DummyFetchExchange {
        Self
    }
}

#[async_trait]
impl Exchange for DummyFetchExchange {
    async fn run<Q: GraphQLQuery>(
        &self,
        operation: Operation<Q::Variables>
    ) -> Result<OperationResult<Q::ResponseData>, Box<dyn Error>> {
        use crate::queries::get_conference::get_conference::{
            GetConferenceConference, ResponseData
        };

        let delay = Duration::from_millis(50);
        tokio::time::delay_for(delay).await;
        let data = Some(ResponseData {
            conference: Some(GetConferenceConference {
                id: "1".to_string(),
                city: Some("Test City".to_string()),
                name: "Test Conference".to_string(),
                talks: Some(Vec::new())
            })
        });

        let data: Box<dyn Any> = Box::new(data);
        let data = *data.downcast::<Q::ResponseData>().unwrap();

        let result = OperationResult {
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

fn main() {
    let url = "http://localhost:8080/graphql";
    let builder = ClientBuilder::new(url)
        .with_exchange(DummyFetchExchange)
        .with_exchange(CacheExchange)
        .with_exchange(DedupExchange);
    let client = Arc::new(builder.build());

    println!("Started");

    let n = 25;
    let variable_set: Vec<Variables> = (0..n).map(|i| Variables { id: i.to_string() }).collect();

    iter::repeat(client).for_each(|client| {
        let var_id = rand::thread_rng().gen_range(0, n);
        let variables = variable_set.get(var_id).cloned().unwrap();
        begin();
        tokio_test::block_on(async move {
            client.query(GetConference, variables).await.unwrap();
        });
        end();
    });
}
