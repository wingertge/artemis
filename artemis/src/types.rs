use crate::{GraphQLQuery, QueryBody, Response, Client, QueryError};
use serde::{de::DeserializeOwned, Serialize};
use std::{sync::Arc};
use surf::url::Url;
use futures::channel::mpsc::Receiver;
use futures::Stream;
use futures::task::Context;
use std::any::Any;
use serde::export::PhantomData;
use std::task::Poll;
use std::pin::Pin;

pub type ExchangeResult<R> = Result<OperationResult<R>, QueryError>;

#[async_trait]
pub trait Exchange: Send + Sync + 'static {
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

#[derive(Clone)]
pub enum FieldSelector {
    /// field name, arguments
    Scalar(String, String),
    /// field_name, arguments, inner selection
    Object(String, String, Vec<FieldSelector>),
    /// field name, arguments, inner selection by type
    Union(String, String, Arc<Box<dyn Fn(&str) -> Vec<FieldSelector>>>)
}

#[derive(Clone, Debug)]
pub struct OperationMeta {
    pub key: u64,
    pub operation_type: OperationType,
    pub involved_types: Vec<&'static str> //pub selection: Vec<FieldSelector>
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

pub struct Observable<T, M: Exchange> {
    inner: Receiver<Arc<dyn Any + Send + Sync>>,
    client: Arc<Client<M>>,
    key: u64,
    index: usize,
    t: PhantomData<T>
}

impl <T: Clone, M: Exchange> Observable<T, M> {
    pub(crate) fn new(key: u64, inner: Receiver<Arc<dyn Any + Send + Sync>>, client: Arc<Client<M>>, index: usize) -> Self {
        Observable {
            inner,
            client,
            key,
            index,
            t: PhantomData
        }
    }
}

impl <T, M: Exchange> Stream for Observable<T, M> where T: 'static + Unpin + Clone {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = &mut self.get_mut().inner;
        let poll = <Receiver<Arc<dyn Any + Send + Sync>> as Stream>::poll_next(Pin::new(inner), cx);
        match poll {
            Poll::Ready(Some(boxed)) => {
                let cast: &T = (&*boxed).downcast_ref::<T>().unwrap();
                Poll::Ready(Some(cast.clone()))
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending
        }
    }
}

impl <T, M: Exchange> Drop for Observable<T, M> {
    fn drop(&mut self) {
        self.client.clear_observable(self.key, self.index)
    }
}