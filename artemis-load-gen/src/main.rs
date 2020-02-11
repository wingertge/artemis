use crate::queries::get_conference::get_conference::Variables;
use crate::queries::get_conference::GetConference;
use rayon::iter;
use rand::Rng;
use std::sync::Arc;
use artemis::{ClientBuilder, FetchExchange};
use rayon::iter::ParallelIterator;
use artemis::exchanges::CacheExchange;

mod queries;

pub(crate) type Long = String;

#[cfg(target_os = "linux")]
fn begin() {
    coz::begin!("query");
}

#[cfg(not(target_os = "linux"))]
fn begin() {

}

#[cfg(target_os = "linux")]
fn end() {
    coz::end!("query")
}

#[cfg(not(target_os = "linux"))]
fn end() {

}

fn main() {
    let url = "http://localhost:8080/graphql";
    let builder = ClientBuilder::new(url)
        .with_exchange(FetchExchange)
        .with_exchange(CacheExchange);
    let client = Arc::new(builder.build());

    let n = 25;
    let variable_set: Vec<Variables> = (0..n)
        .map(|i| Variables { id: i.to_string() })
        .collect();

    iter::repeat(client)
        .for_each(|client| {
            let var_id = rand::thread_rng().gen_range(0, n);
            let variables = variable_set.get(var_id).cloned().unwrap();
            begin();
            tokio_test::block_on(async move {
                client.query(GetConference, variables).await.unwrap();
            });
            end();
        });
}
