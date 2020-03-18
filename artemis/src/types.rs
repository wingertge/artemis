use crate::{client::ClientImpl, GraphQLQuery, QueryBody, QueryError, Response};
use futures::{channel::mpsc::Receiver, task::Context, Stream};
use serde::{de::DeserializeOwned, export::PhantomData, Serialize};
use std::{any::Any, collections::HashMap, pin::Pin, sync::Arc, task::Poll};
use surf::url::Url;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use type_map::concurrent::TypeMap;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsValue, JsCast};
use parking_lot::RwLock;

pub type ExchangeResult<R> = Result<OperationResult<R>, QueryError>;

#[async_trait]
pub trait Exchange: Send + Sync + 'static {
    async fn run<Q: GraphQLQuery, C: crate::exchanges::Client>(
        &self,
        operation: Operation<Q::Variables>,
        client: C
    ) -> ExchangeResult<Q::ResponseData>;
}

pub trait ExchangeFactory<TNext: Exchange> {
    type Output: Exchange;

    fn build(self, next: TNext) -> Self::Output;
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

impl ToString for OperationType {
    fn to_string(&self) -> String {
        let str = match self {
            OperationType::Query => "Query",
            OperationType::Mutation => "Mutation",
            OperationType::Subscription => "Subscription"
        };
        str.to_string()
    }
}

#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum RequestPolicy {
    CacheFirst = 1,
    CacheOnly = 2,
    NetworkOnly = 3,
    CacheAndNetwork = 4
}

impl From<u8> for RequestPolicy {
    fn from(value: u8) -> Self {
        match value {
            1 => RequestPolicy::CacheFirst,
            2 => RequestPolicy::CacheOnly,
            3 => RequestPolicy::NetworkOnly,
            4 => RequestPolicy::CacheAndNetwork,
            _ => unreachable!()
        }
    }
}

pub struct HeaderPair(pub String, pub String);

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
    pub query_key: u32,
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
    pub key: u64,
    pub meta: OperationMeta,
    pub query: QueryBody<V>,
    pub options: OperationOptions
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ResultSource {
    Cache,
    Network
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[derive(Clone, Debug, PartialEq)]
pub struct DebugInfo {
    pub source: ResultSource,
    pub did_dedup: bool
}

#[derive(Clone, Debug)]
pub struct OperationResult<R: DeserializeOwned + Send + Sync + Clone> {
    pub key: u64,
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

pub trait Extension: Sized + Clone + Send + Sync + 'static {
    #[cfg(target_arch = "wasm32")]
    fn from_js(value: JsValue) -> Option<Self>;
}

pub struct ExtensionMap {
    rust: TypeMap,
    #[cfg(target_arch = "wasm32")]
    js: JsValue
}

// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl Send for ExtensionMap {}
// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl Sync for ExtensionMap {}

impl ExtensionMap {
    pub fn new() -> Self {
        Self {
            rust: TypeMap::new(),
            #[cfg(target_arch = "wasm32")]
            js: JsValue::NULL
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_js(value: JsValue) -> Option<Self> {
        if value.is_object() {
            Some(Self {
                rust: TypeMap::new(),
                js: value
            })
        } else {
            None
        }
    }

    pub fn insert<T: Extension>(&mut self, value: T) {
        self.rust.insert(value);
    }

    pub fn get<T: Extension>(&self, js_key: impl Into<String>) -> Option<T> {
        self.get_rust()
            .or_else(|| self.get_js(js_key.into()))
    }

    #[cfg(target_arch = "wasm32")]
    fn get_js<T: Extension>(&self, js_key: String) -> Option<T> {
        let key: JsValue = js_key.clone().into();
        js_sys::Reflect::get(&self.js, &key).ok()
            .and_then(|value| T::from_js(value))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get_js<T: Extension>(&self, js_key: String) -> Option<T> {
        None
    }

    fn get_rust<T: Extension>(&self) -> Option<T> {
        self.rust.get::<T>().cloned()
    }
}

pub type Extensions = Arc<ExtensionMap>;

#[derive(Default, Clone)]
pub struct QueryOptions {
    pub url: Option<Url>,
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    pub request_policy: Option<RequestPolicy>,
    pub extensions: Option<Extensions>
}
