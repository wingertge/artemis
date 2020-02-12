#![feature(custom_test_frameworks)]
#![test_runner(criterion::runner)]

use artemis::{
    exchanges::{CacheExchange, DedupExchange},
    ClientBuilder, Exchange, ExchangeFactory, Operation, OperationResult, Response
};
use artemis_test::get_conference::{get_conference::Variables, GetConference};
use async_trait::async_trait;
use criterion::{Criterion, Throughput};
use criterion_macro::criterion;
use rand::Rng;
use serde::Serialize;
use std::{error::Error, time::Duration};

struct DummyFetchExchange;

impl<TNext: Exchange> ExchangeFactory<DummyFetchExchange, TNext> for DummyFetchExchange {
    fn build(_next: TNext) -> DummyFetchExchange {
        Self
    }
}

const N_CONCURRENCY: u64 = 100;

#[async_trait]
impl Exchange for DummyFetchExchange {
    async fn run<V: Serialize + Send + Sync>(
        &self,
        operation: Operation<V>
    ) -> Result<OperationResult, Box<dyn Error>> {
        use artemis_test::get_conference::get_conference::{GetConferenceConference, ResponseData};

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

        let result = OperationResult {
            meta: operation.meta,
            response: Response {
                data: Some(serde_json::to_value(data).unwrap()),
                debug_info: None,
                errors: None
            }
        };

        Ok(result)
    }
}

#[criterion(create_criterion())]
pub fn benchmark_random_queries(c: &mut Criterion) {
    let url = "http://localhost:8080/graphql";
    let builder = ClientBuilder::new(url)
        .with_exchange(DummyFetchExchange)
        .with_exchange(CacheExchange)
        .with_exchange(DedupExchange);
    let client = builder.build();
    let mut rand = rand::thread_rng();

    let mut group = c.benchmark_group("throughput-random");
    group.throughput(Throughput::Elements(N_CONCURRENCY));

    group.bench_function("random queries", |bencher| {
        bencher.iter(|| {
            let futures = (0..N_CONCURRENCY).map(|_| {
                let id: u32 = rand.gen();
                let variables = Variables { id: id.to_string() };
                client.query(GetConference, variables)
            });
            let all = futures::future::join_all(futures);
            tokio_test::block_on(all);
        })
    });
}

#[criterion(create_criterion())]
pub fn benchmark_cached_queries(c: &mut Criterion) {
    let url = "http://localhost:8080/graphql";
    let builder = ClientBuilder::new(url)
        .with_exchange(DummyFetchExchange)
        .with_exchange(CacheExchange);
    let client = builder.build();

    let mut rand = rand::thread_rng();

    let n = (N_CONCURRENCY / 4) as usize;
    let variable_set: Vec<Variables> = (0..n).map(|i| Variables { id: i.to_string() }).collect();

    let mut group = c.benchmark_group("throughput-cached");
    group.throughput(Throughput::Elements(N_CONCURRENCY));

    group.bench_function("cached queries", |bencher| {
        bencher.iter(|| {
            let futures = (0..N_CONCURRENCY).map(|_| {
                let var_id: usize = rand.gen_range(0, n);
                let variables = variable_set.get(var_id).unwrap().clone();
                client.query(GetConference, variables)
            });
            let all = futures::future::join_all(futures);
            tokio_test::block_on(all);
        })
    });
}

#[criterion(create_criterion())]
pub fn benchmark_real_world(c: &mut Criterion) {
    let url = "http://localhost:8080/graphql";
    let builder = ClientBuilder::new(url)
        .with_exchange(DummyFetchExchange)
        .with_exchange(CacheExchange)
        .with_exchange(DedupExchange);
    let client = builder.build();

    let n = 20;
    // Simulate n different queries run randomly multiple times
    let variable_set: Vec<Variables> = (0..n).map(|i| Variables { id: i.to_string() }).collect();
    let mut rand = rand::thread_rng();

    let mut group = c.benchmark_group("throughput-real");
    group.throughput(Throughput::Elements(N_CONCURRENCY));

    group.bench_function("real world queries", |bencher| {
        bencher.iter(|| {
            let futures = (0..N_CONCURRENCY).map(|_| {
                let var_id: usize = rand.gen_range(0, n);
                let variables = variable_set.get(var_id).unwrap().clone();
                client.query(GetConference, variables)
            });
            let all = futures::future::join_all(futures);
            tokio_test::block_on(all);
        })
    });
}

fn create_criterion() -> Criterion {
    Criterion::default().sample_size(10)
}
