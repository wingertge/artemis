use crate::types::{Middleware, Operation, OperationResult};
use serde::Serialize;
use std::{error::Error, fmt};

mod cache;
pub mod fetch;

pub use fetch::FetchMiddleware;
pub use cache::CacheMiddleware;

#[derive(Debug)]
enum MiddlewareError {
    UnexpectedEndOfChain
}
impl Error for MiddlewareError {}

impl fmt::Display for MiddlewareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unexpected end of middleware chain")
    }
}

pub struct DummyMiddleware;

#[async_trait]
impl Middleware for DummyMiddleware {
    async fn run<V: Serialize + Send + Sync>(
        &self,
        _operation: Operation<V>
    ) -> Result<OperationResult, Box<dyn Error>> {
        Err(MiddlewareError::UnexpectedEndOfChain.into())
    }
}
