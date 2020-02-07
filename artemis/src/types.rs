use crate::QueryBody;
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt};
use surf::url::Url;

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

#[async_trait]
pub trait Middleware {
    async fn run<
        T: Serialize + DeserializeOwned + Send + Sync,
        F: Fn() -> Vec<HeaderPair> + Send + Sync
    >(
        &self,
        operation: Operation<T, F>
    ) -> Result<OperationResult, Box<dyn Error>>;
}

pub trait MiddlewareFactory<TNext: Middleware + Send + Sync> {
    fn build(next: TNext) -> Self;
}

pub enum OperationType {
    Query,
    Mutation,
    Subscription
}

pub enum RequestPolicy {
    CacheFirst,
    CacheOnly,
    NetworkOnly,
    CacheAndNetwork
}

pub struct HeaderPair(pub &'static str, pub &'static str);

pub struct Operation<T: Serialize + DeserializeOwned, F: Fn() -> Vec<HeaderPair>> {
    pub operation_type: OperationType,
    pub query: QueryBody<T>,
    pub url: Url,
    pub request_policy: RequestPolicy,
    pub extra_headers: Option<F>
}

pub struct OperationResult {
    pub response_string: String
}
