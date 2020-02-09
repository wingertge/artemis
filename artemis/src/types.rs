use crate::QueryBody;
use serde::Serialize;
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

#[derive(PartialEq, Debug, Clone)]
pub enum OperationType {
    Query,
    Mutation,
    Subscription
}

impl From<u8> for OperationType {
    fn from(u: u8) -> Self {
        match u {
            1 => OperationType::Mutation,
            2 => OperationType::Subscription,
            _ => OperationType::Query
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestPolicy {
    CacheFirst,
    CacheOnly,
    NetworkOnly,
    CacheAndNetwork
}

pub struct HeaderPair(pub &'static str, pub &'static str);

pub struct OperationMeta {
    pub key: u32,
    pub operation_type: OperationType,
    pub involved_types: Option<Vec<&'static str>>
}

pub struct Operation<V: Serialize> {
    pub meta: OperationMeta,
    pub query: QueryBody<V>,
    pub url: Url,
    pub request_policy: RequestPolicy,
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>
}

pub struct OperationResult {
    pub meta: OperationMeta,
    pub response_string: String
}
