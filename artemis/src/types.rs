use crate::{client::ClientImpl, GraphQLQuery, QueryBody, QueryError, Response};
use futures::{channel::mpsc::Receiver, task::Context, Stream};
use serde::{de::DeserializeOwned, export::PhantomData, Serialize};
use std::{any::Any, pin::Pin, sync::Arc, task::Poll};
use surf::url::Url;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub type ExchangeResult<R> = Result<OperationResult<R>, QueryError>;

#[async_trait]
pub trait Exchange: Send + Sync + 'static {
    async fn run<Q: GraphQLQuery, C: crate::exchanges::Client>(
        &self,
        operation: Operation<Q::Variables>,
        client: C
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
    /// field_name, arguments, optional, inner selection
    Object(String, String, bool, Vec<FieldSelector>),
    /// field name, arguments, optional, inner selection by type
    Union(
        String,
        String,
        bool,
        Arc<Box<dyn Fn(&str) -> Vec<FieldSelector>>>
    )
}

#[derive(Clone, Debug)]
pub struct OperationMeta {
    pub key: u64,
    pub operation_type: OperationType,
    pub involved_types: Vec<&'static str> //pub selection: Vec<FieldSelector>
}

#[derive(Clone)]
pub struct OperationOptions {
    pub url: Url,
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    pub request_policy: RequestPolicy,
    pub extensions: Option<Extensions>
}

#[derive(Clone)]
pub struct Operation<V: Serialize + Clone + Send + Sync> {
    pub meta: OperationMeta,
    pub query: QueryBody<V>,
    pub options: OperationOptions
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResultSource {
    Cache,
    Network
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct DebugInfo {
    pub source: ResultSource,
    pub did_dedup: bool
}

#[derive(Clone, Debug)]
pub struct OperationResult<R: DeserializeOwned + Send + Sync + Clone> {
    pub meta: OperationMeta,
    pub response: Response<R>
}

#[cfg(feature = "observable")]
pub struct Observable<T, M: Exchange> {
    inner: Receiver<Arc<dyn Any + Send + Sync>>,
    client: Arc<ClientImpl<M>>,
    key: u64,
    index: usize,
    t: PhantomData<T>
}

#[cfg(feature = "observable")]
impl<T: Clone, M: Exchange> Observable<T, M> {
    pub(crate) fn new(
        key: u64,
        inner: Receiver<Arc<dyn Any + Send + Sync>>,
        client: Arc<ClientImpl<M>>,
        index: usize
    ) -> Self {
        Observable {
            inner,
            client,
            key,
            index,
            t: PhantomData
        }
    }
}

#[cfg(feature = "observable")]
impl<T, M: Exchange> Stream for Observable<T, M>
where
    T: 'static + Unpin + Clone
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let inner = &mut self.get_mut().inner;
        let poll = <Receiver<Arc<dyn Any + Send + Sync>> as Stream>::poll_next(Pin::new(inner), cx);
        match poll {
            Poll::Ready(Some(boxed)) => {
                let cast: &T = (&*boxed).downcast_ref::<T>().unwrap();
                Poll::Ready(Some(cast.clone()))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending
        }
    }
}

#[cfg(feature = "observable")]
impl<T, M: Exchange> Drop for Observable<T, M> {
    fn drop(&mut self) {
        self.client.clear_observable(self.key, self.index)
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub type Extensions = Arc<type_map::concurrent::TypeMap>;

#[derive(Default, Clone)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct QueryOptions {
    pub url: Option<Url>,
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    pub request_policy: Option<RequestPolicy>,
    pub extensions: Option<Extensions>
}
