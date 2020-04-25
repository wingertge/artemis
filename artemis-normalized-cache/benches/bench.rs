use crate::queries::{
    books::{books, Books},
    complex_author::{
        complex_author,
        complex_author::{ComplexBook, ComplexReview, ComplexReviewer},
        ComplexAuthor
    },
    employees::{employees, Employees},
    stores::{stores, Stores},
    todos::{todos_query, TodosQuery},
    writers::{writers, Writers}
};
use artemis::{
    exchange::{Operation, OperationMeta, OperationOptions, OperationResult, OperationType},
    GraphQLQuery, QueryBody, RequestPolicy, Response
};
use artemis_normalized_cache::{HashSet, Store};
use chrono::{Duration, Utc};
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
    Throughput
};
use rand::Rng;
use std::collections::HashMap;

mod queries;

criterion_group!(benches, write);
criterion_main!(benches);

pub fn read(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");

    benchmark_reads(&mut group, 100);
    benchmark_reads(&mut group, 1000);
    benchmark_reads(&mut group, 10000);

    group.finish();
}

pub fn write(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");

    benchmark_writes(&mut group, 100);
    /*
    benchmark_writes(&mut group, 1000);
    benchmark_writes(&mut group, 10000);

    benchmark_write_five_entities(&mut group, 100);
    benchmark_write_five_entities(&mut group, 1000);
    benchmark_write_five_entities(&mut group, 10000);

    benchmark_write_complex(&mut group, 100);
    benchmark_write_complex(&mut group, 1000);
    benchmark_write_complex(&mut group, 10000);
    */

    group.finish();
}

fn make_operation_result<Q: GraphQLQuery>(
    data: Q::ResponseData
) -> OperationResult<Q::ResponseData> {
    OperationResult {
        key: 1,
        meta: OperationMeta {
            query_key: 1,
            operation_type: OperationType::Query,
            involved_types: Vec::new()
        },
        response: Response {
            data: Some(data),
            debug_info: None,
            errors: None
        }
    }
}

fn make_todo(i: usize) -> todos_query::Todo {
    todos_query::Todo {
        id: i.to_string(),
        due: (Utc::now() - Duration::milliseconds(rand::random::<u32>() as i64)).to_rfc3339(),
        text: format!("Todo {}", i),
        complete: i % 2 == 0
    }
}

fn make_todos(n: usize) -> OperationResult<todos_query::ResponseData> {
    let todos = (0..n).map(make_todo).collect();
    make_operation_result::<TodosQuery>(todos_query::ResponseData { todos })
}

fn make_read_op<Q: GraphQLQuery>(
    query: &'static str,
    variables: Q::Variables
) -> Operation<Q::Variables> {
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
            query,
            variables,
            operation_name: ""
        }
    }
}

fn benchmark_reads(group: &mut BenchmarkGroup<WallTime>, n: usize) {
    use todos_query::{Variables, QUERY};

    let store = Store::new(HashMap::default());
    let data = make_todos(n);

    let mut deps = HashSet::default();
    store
        .write_query::<TodosQuery>(&data, &Variables, false, &mut deps)
        .unwrap();

    let operation = make_read_op::<TodosQuery>(QUERY, Variables);

    group.throughput(Throughput::Elements(n as u64));
    group.sample_size(usize::max(10, 10000 / n));
    group.bench_with_input(
        BenchmarkId::new("one entity", format!("{} entries", n)),
        &operation,
        |b, op| {
            b.iter(|| {
                store
                    .read_query::<TodosQuery>(op, std::ptr::null_mut())
                    .unwrap()
            });
        }
    );
}

fn benchmark_writes(group: &mut BenchmarkGroup<WallTime>, n: usize) {
    use todos_query::Variables;

    let store = Store::new(HashMap::default());
    let data = make_todos(n);

    group.throughput(Throughput::Elements(n as u64));
    group.sample_size(usize::max(10, 10000 / n));
    group.bench_with_input(
        BenchmarkId::new("one entity", format!("{} entries", n)),
        &data,
        |b, op| {
            let mut deps = HashSet::default();
            b.iter(|| {
                store
                    .write_query::<TodosQuery>(op, &Variables, false, &mut deps)
                    .unwrap()
            });
        }
    );
}

const COUNTRIES: [&'static str; 4] = ["UK", "BE", "ES", "US"];

fn make_writers(n: usize) -> OperationResult<writers::ResponseData> {
    let mut rand = rand::thread_rng();
    let make_writer = |i: usize| writers::Writer {
        id: i.to_string(),
        name: format!("writer {}", i),
        amount_of_books: rand.gen_range(0, 100),
        recognised: i % 2 == 0,
        number: i as i64,
        interests: "star wars".to_string()
    };
    let entries = (0..n).map(make_writer).collect();
    make_operation_result::<Writers>(writers::ResponseData { writers: entries })
}

fn make_books(n: usize) -> OperationResult<books::ResponseData> {
    let mut rand = rand::thread_rng();
    let make_book = |i: usize| books::Book {
        id: i.to_string(),
        title: format!("book {}", i),
        published: i % 2 == 0,
        genre: "Fantasy".to_string(),
        rating: rand.gen_range(0, 100),
        release: (Utc::now() - Duration::milliseconds(rand.gen::<u32>() as i64)).to_rfc3339()
    };
    let entries = (0..n).map(make_book).collect();
    make_operation_result::<Books>(books::ResponseData { books: entries })
}

fn make_stores(n: usize) -> OperationResult<stores::ResponseData> {
    let mut rand = rand::thread_rng();
    let make_store = |i: usize| stores::Store {
        id: i.to_string(),
        name: format!("store {}", i),
        started: (Utc::now() - Duration::milliseconds(rand.gen::<u32>() as i64)).to_rfc3339(),
        country: COUNTRIES[rand.gen_range(0, 4)].to_string()
    };
    let entries = (0..n).map(make_store).collect();
    make_operation_result::<Stores>(stores::ResponseData { stores: entries })
}

fn make_employees(n: usize) -> OperationResult<employees::ResponseData> {
    let mut rand = rand::thread_rng();
    let make_employee = |i: usize| employees::Employee {
        id: i.to_string(),
        name: format!("employee {}", i),
        date_of_birth: (Utc::now() - Duration::milliseconds(rand.gen::<u32>() as i64)).to_rfc3339(),
        origin: COUNTRIES[rand.gen_range(0, 4)].to_string()
    };
    let entries = (0..n).map(make_employee).collect();
    make_operation_result::<Employees>(employees::ResponseData { employees: entries })
}

fn benchmark_write_five_entities(group: &mut BenchmarkGroup<WallTime>, n: usize) {
    let store = Store::new(HashMap::default());

    let books = make_books(n);
    let employees = make_employees(n);
    let stores = make_stores(n);
    let writers = make_writers(n);
    let todos = make_todos(n);
    let data = (books, employees, stores, writers, todos);

    group.throughput(Throughput::Elements((n * 5) as u64));
    group.sample_size(usize::max(10, 2000 / n));
    group.bench_with_input(
        BenchmarkId::new("five entities", format!("{} entries", n)),
        &data,
        |b, data| {
            let mut deps = HashSet::default();
            let (books, employees, stores, writers, todos) = data;
            b.iter(|| {
                store
                    .write_query::<Books>(books, &books::Variables, false, &mut deps)
                    .unwrap();
                store
                    .write_query::<Employees>(employees, &employees::Variables, false, &mut deps)
                    .unwrap();
                store
                    .write_query::<Stores>(stores, &stores::Variables, false, &mut deps)
                    .unwrap();
                store
                    .write_query::<Writers>(writers, &writers::Variables, false, &mut deps)
                    .unwrap();
                store
                    .write_query::<TodosQuery>(todos, &todos_query::Variables, false, &mut deps)
                    .unwrap()
            })
        }
    );
}

fn make_authors(n: usize) -> OperationResult<complex_author::ResponseData> {
    let make_author = |i: usize| complex_author::ComplexAuthor {
        id: i.to_string(),
        name: format!("author {}", i),
        recognised: i % 2 == 0,
        book: ComplexBook {
            id: i.to_string(),
            name: format!("book {}", i),
            published: i % 2 == 0,
            review: ComplexReview {
                id: i.to_string(),
                score: i as i64,
                name: format!("review {}", i),
                reviewer: ComplexReviewer {
                    id: i.to_string(),
                    name: format!("person {}", i),
                    verified: i % 2 == 0
                }
            }
        }
    };
    let authors = (0..n).map(make_author).collect();

    make_operation_result::<ComplexAuthor>(complex_author::ResponseData { authors })
}

fn benchmark_write_complex(group: &mut BenchmarkGroup<WallTime>, n: usize) {
    use complex_author::Variables;

    let store = Store::new(HashMap::default());
    let data = make_authors(n);

    group.throughput(Throughput::Elements(n as u64));
    group.sample_size(usize::max(10, 10000 / n));
    group.bench_with_input(
        BenchmarkId::new("complex entity", format!("{} entries", n)),
        &data,
        |b, op| {
            let mut deps = HashSet::default();
            b.iter(|| {
                store
                    .write_query::<ComplexAuthor>(op, &Variables, false, &mut deps)
                    .unwrap()
            });
        }
    );
}

/*pub fn writes_five_entities(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");

    benchmark_writes_five_entities(&mut group, 100);
    benchmark_writes_five_entities(&mut group, 1000);
    benchmark_writes_five_entities(&mut group, 10000);

    group.finish();
}*/
