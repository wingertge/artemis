use crate::types::{Middleware, Operation, RequestPolicy, Next, OperationResult};
use crate::Response;
use std::error::Error;
use crate::middlewares::DummyMiddleware;
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::any::Any;

type ResultCache = Arc<Mutex<HashMap<u32, OperationResult>>>;
type OperationCache = Arc<Mutex<HashMap<String, HashSet<u32>>>>;

#[derive(Default)]
struct CacheMiddleware {
    result_cache: ResultCache,
    operation_cache: OperationCache
}

fn map_type_names<T>(operation: Operation<T>) -> Operation<T> {
    // TODO: Add __typename to query automatically
    operation
}

impl CacheMiddleware {
    pub fn new() -> Self {
        Self {
            result_cache: Arc::new(Mutex::new(HashMap::new())),
            operation_cache: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    fn after_mutation(&self, response: OperationResult) {
        let pending_operations = HashSet::new();
    }
}

#[async_trait]
impl Middleware for CacheMiddleware {
    async fn run<T>(&mut self, operation: Operation<T>, next: &mut Next) -> Result<Response<T>, Box<dyn Error>> {
        match operation.request_policy {
            RequestPolicy::NetworkOnly => next.run(operation),
            RequestPolicy::CacheAndNetwork => {

            }
        }
    }
}