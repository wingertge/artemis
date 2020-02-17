use crate::types::{Exchange, Operation, OperationResult};
use std::{error::Error, fmt};

mod cache;
mod dedup;
mod fetch;

use crate::GraphQLQuery;
pub use cache::CacheExchange;
pub use dedup::DedupExchange;
pub use fetch::FetchExchange;

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

pub struct DummyExchange;

#[async_trait]
impl Exchange for DummyExchange {
    async fn run<Q: GraphQLQuery>(
        &self,
        _operation: Operation<Q::Variables>
    ) -> Result<OperationResult<Q::ResponseData>, Box<dyn Error>> {
        Err(MiddlewareError::UnexpectedEndOfChain.into())
    }
}
