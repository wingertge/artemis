use crate::{GraphQLQuery, QueryBody, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, sync::Arc};
use surf::url::Url;

pub type ExchangeResult<R> = Result<OperationResult<R>, Box<dyn Error>>;

#[async_trait]
pub trait Exchange: Send + Sync {
    async fn run<Q: GraphQLQuery>(
        &self,
        operation: Operation<Q::Variables>
    ) -> ExchangeResult<Q::ResponseData>;
}

pub trait ExchangeFactory<T: Exchange, TNext: Exchange> {
    fn build(self, next: TNext) -> T;
}

pub trait QueryInfo<TVars> {
    fn typename(&self) -> &'static str;
    fn selection(variables: &TVars) -> Vec<FieldSelector>;
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

#[derive(Clone, Debug)]
pub enum FieldSelector {
    Scalar(String),
    Object(String, Vec<FieldSelector>)
}

#[derive(Clone, Debug)]
pub struct OperationMeta {
    pub key: u64,
    pub operation_type: OperationType,
    pub involved_types: Vec<&'static str>
    //pub selection: Vec<FieldSelector>
}

#[derive(Clone)]
pub struct Operation<V: Serialize + Clone + Send + Sync> {
    pub meta: OperationMeta,
    pub query: QueryBody<V>,
    pub url: Url,
    pub request_policy: RequestPolicy,
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResultSource {
    Cache,
    Network
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebugInfo {
    pub source: ResultSource,
    pub did_dedup: bool
}

#[derive(Clone, Debug)]
pub struct OperationResult<R: DeserializeOwned + Send + Sync + Clone> {
    pub meta: OperationMeta,
    pub response: Response<R>
}
