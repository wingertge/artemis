use artemis::{
    exchange::{Operation, OperationMeta, OperationOptions, OperationResult, OperationType},
    QueryBody, RequestPolicy, Response
};
use artemis_normalized_cache::{HashSet, Store};
use artemis_test::{
    get_conferences::get_conferences::ResponseData,
    queries::get_conferences::{
        get_conferences::{GetConferencesConferences, Variables},
        GetConferences
    }
};
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
    Throughput
};
use std::collections::HashMap;

fn make_data(n: usize) -> OperationResult<ResponseData> {
    let mut entities = Vec::with_capacity(n);
    for i in 0..n {
        let entity = GetConferencesConferences {
            id: i.to_string(),
            name: format!("Conference {}", i),
            city: Some(format!("City {}", i)),
            talks: Some(Vec::new())
        };
        entities.push(entity);
    }
    OperationResult {
        key: 1,
        meta: OperationMeta {
            query_key: 1,
            operation_type: OperationType::Query,
            involved_types: Vec::new()
        },
        response: Response {
            data: Some(ResponseData {
                conferences: Some(entities)
            }),
            debug_info: None,
            errors: None
        }
    }
}

fn make_read_op() -> Operation<Variables> {
    Operation {
        key: 1,
        meta: OperationMeta {
            query_key: 1,
            operation_type: OperationType::Query,
            involved_types: Vec::new()
        },
        options: OperationOptions {
            request_policy: RequestPolicy::CacheOnly,
            url: "".to_string(),
            extensions: None,
            extra_headers: None
        },
        query: QueryBody {
            query: artemis_test::get_conferences::get_conferences::QUERY,
            variables: Variables,
            operation_name: ""
        }
    }
}

fn benchmark_reads(group: &mut BenchmarkGroup<WallTime>, n: usize) {
    let store = Store::new(HashMap::new());
    let data = make_data(n);

    let mut deps = HashSet::default();
    store
        .write_query::<GetConferences>(&data, &Variables, false, &mut deps)
        .unwrap();

    let operation = make_read_op();

    group.throughput(Throughput::Elements(n as u64));
    group.sample_size(1000 * 100 / n);
    group.bench_with_input(
        BenchmarkId::new(n.to_string(), "queries"),
        &operation,
        |b, op| {
            b.iter(|| {
                store
                    .read_query::<GetConferences>(op, std::ptr::null_mut())
                    .unwrap()
            });
        }
    );
}

pub fn reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("reads");

    benchmark_reads(&mut group, 100);
    benchmark_reads(&mut group, 1000);
    benchmark_reads(&mut group, 10000);

    group.finish();
}

fn benchmark_writes(group: &mut BenchmarkGroup<WallTime>, n: usize) {
    let store = Store::new(HashMap::new());
    let data = make_data(n);

    group.throughput(Throughput::Elements(n as u64));
    group.sample_size(10);
    group.bench_with_input(
        BenchmarkId::new(n.to_string(), "queries"),
        &data,
        |b, op| {
            let mut deps = HashSet::default();
            b.iter(|| {
                store
                    .write_query::<GetConferences>(op, &Variables, false, &mut deps)
                    .unwrap()
            });
        }
    );
}

pub fn writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("writes");

    benchmark_writes(&mut group, 100);
    benchmark_writes(&mut group, 1000);
    benchmark_writes(&mut group, 10000);

    group.finish();
}

/*fn writes_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("writes");

    benchmark_writes(&mut group, 100);

    group.finish();
}*/

criterion_group!(benches, reads, writes);
criterion_main!(benches);
