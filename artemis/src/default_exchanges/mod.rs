//! This module contains the default exchanges.
//! Note that these require the `default-exchanges` feature.

use crate::types::{Exchange, Operation};
use std::{error::Error, fmt};

#[cfg(feature = "default-exchanges")]
mod cache;
#[cfg(feature = "default-exchanges")]
mod dedup;
#[cfg(feature = "default-exchanges")]
mod fetch;

use crate::{exchange::Client, ExchangeResult, GraphQLQuery};
#[cfg(feature = "default-exchanges")]
pub use cache::CacheExchange;
#[cfg(feature = "default-exchanges")]
pub use dedup::DedupExchange;
#[cfg(feature = "default-exchanges")]
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

/// The terminating exchange.
/// This will always be the last exchange in the chain and will simply return an error if called.
pub struct TerminatorExchange;

#[async_trait]
impl Exchange for TerminatorExchange {
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        _operation: Operation<Q::Variables>,
        _client: C
    ) -> ExchangeResult<Q::ResponseData> {
        Err(MiddlewareError::UnexpectedEndOfChain.into())
    }
}
