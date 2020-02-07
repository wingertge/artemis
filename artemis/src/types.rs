use crate::QueryBody;
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, sync::Arc};
use surf::url::Url;

#[async_trait]
pub trait Middleware {
    async fn run<V: Serialize + Send + Sync>(
        &self,
        operation: Operation<V>
    ) -> Result<OperationResult, Box<dyn Error>>;
}

pub trait MiddlewareFactory<T: Middleware + Send + Sync, TNext: Middleware + Send + Sync> {
    fn build(next: TNext) -> T;
}

pub enum OperationType {
    Query,
    Mutation,
    Subscription
}

#[derive(Debug, Clone)]
pub enum RequestPolicy {
    CacheFirst,
    CacheOnly,
    NetworkOnly,
    CacheAndNetwork
}

pub struct HeaderPair(pub &'static str, pub &'static str);

pub struct Operation<V: Serialize> {
    pub query: QueryBody<V>,
    pub url: Url,
    pub request_policy: RequestPolicy,
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>
}

pub struct OperationResult {
    pub response_string: String
}
