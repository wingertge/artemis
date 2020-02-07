use crate::types::{HeaderPair, Middleware, Operation, OperationResult};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt};

pub mod fetch;
//mod cache;

pub use fetch::FetchMiddleware;

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
        operation: Operation<V>
    ) -> Result<OperationResult, Box<dyn Error>> {
        Err(MiddlewareError::UnexpectedEndOfChain.into())
    }
}
