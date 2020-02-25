use crate::types::{Exchange, Operation};
use std::{error::Error, fmt};

#[cfg(feature = "default-exchanges")]
mod cache;
#[cfg(feature = "default-exchanges")]
mod dedup;
#[cfg(feature = "default-exchanges")]
mod fetch;

use crate::{client::ClientImpl, ExchangeResult, GraphQLQuery};
#[cfg(feature = "default-exchanges")]
pub use cache::CacheExchange;
#[cfg(feature = "default-exchanges")]
pub use dedup::DedupExchange;
#[cfg(feature = "default-exchanges")]
pub use fetch::FetchExchange;
use std::sync::Arc;

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
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        _operation: Operation<Q::Variables>,
        _client: C
    ) -> ExchangeResult<Q::ResponseData> {
        Err(MiddlewareError::UnexpectedEndOfChain.into())
    }
}

/// Client trait passed to exchanges. Only exposes methods useful to exchanges
pub trait Client: Clone + Send + Sync + 'static {
    fn rerun_query(&self, query_key: u64);
}
