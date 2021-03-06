use crate::{client::ClientImpl, GraphQLQuery, QueryBody, QueryError, Response};
#[cfg(feature = "observable")]
use futures::{channel::mpsc::Receiver, task::Context, Stream};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    pin::Pin,
    sync::Arc,
    task::Poll
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
use std::marker::PhantomData;

/// The result type returned by exchanges
pub type ExchangeResult<R> = Result<OperationResult<R>, QueryError>;

/// The main trait that must be implemented by exchanges
#[async_trait]
pub trait Exchange: Send + Sync + 'static {
    /// Process a query operation.
    ///
    /// Note that this can return early, perhaps partial results to subscriptions via `client.push_result()`,
    /// but this will do nothing for regular requests. The exchange should have a workflow in the style of:
    /// Determine if this exchange can return a result -> Delegate to `next` if not -> Perform processing on the delegated result -> Return result
    ///
    /// For examples, see the built-in exchanges.
    ///
    /// # Arguments
    ///
    /// * `operation` - the operation to be handled.
    /// * `client` - the client object for rerunning queries and pushing results if applicable.
    ///
    /// # Returns
    ///
    /// * `Ok(Response)` if the operation was completed successfully
    /// * `Err(YourError)` if there was an error anywhere. Errors from `next` should be passed through.
    /// This will be displayed to the user, so make sure the error messages are reasonable.
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        operation: Operation<Q::Variables>,
        client: C
    ) -> ExchangeResult<Q::ResponseData>;
}

/// An exchange factory. This must be passed to the ClientBuilder by the user,
/// so it should take only necessary parameters.
///
/// In a scenario that doesn't require additional configuration this should just be a zero size struct
pub trait ExchangeFactory<TNext: Exchange> {
    type Output: Exchange;

    /// Build the exchange using the provided `next` exchange. If a query isn't handled
    /// it should be delegated to this exchange.
    fn build(self, next: TNext) -> Self::Output;
}

/// Internal struct used in codegen
pub trait QueryInfo<TVars> {
    /// Recursively creates the selection for the given set of variables.
    /// The variables are used to fill in non-const argument values.
    fn selection(variables: &TVars) -> Vec<FieldSelector>;
}

/// The type of the operation. This corresponds directly to the GraphQL syntax,
/// `query`, `mutation` and `subscription`.
#[derive(PartialEq, Debug, Clone)]
pub enum OperationType {
    Query,
    Mutation,
    Subscription
}

impl OperationType {
    pub fn to_str(&self) -> &'static str {
        match self {
            OperationType::Query => "Query",
            OperationType::Mutation => "Mutation",
            OperationType::Subscription => "Subscription"
        }
    }
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

/// The request policy of the request.
///
/// * `CacheFirst` - Prefers results from the cache, if it's not found it is fetched
/// * `CacheOnly` - Only fetches results from the cache, if it's not found it will simply return `None` for the data
/// * `NetworkOnly` - Only fetches results from the network and ignores the cache.
/// * `CacheAndNetwork` - Returns the result from the cache if it exists, but also refetch from the network and push the result to a subscription.
/// This acts the same as CacheFirst without subscriptions, but has overhead.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq)]
pub enum RequestPolicy {
    /// Prefers results from the cache, if it's not found it is fetched
    CacheFirst = 1,
    /// Only fetches results from the cache, if it's not found it will simply return `None` for the data
    CacheOnly = 2,
    /// Only fetches results from the network and ignores the cache.
    NetworkOnly = 3,
    /// Returns the result from the cache if it exists, but also refetch from the network
    /// and push the result to a subscription. This acts the same as CacheFirst without
    /// subscriptions, but has overhead.
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

/// A key-value pair used for custom headers.
pub struct HeaderPair(pub String, pub String);

/// An internal struct used in codegen.
/// This represents the recursive selection of a query and is used for normalization.
#[derive(Clone)]
pub enum FieldSelector {
    /// field name, arguments
    Scalar(&'static str, String),
    /// field_name, arguments, typename, inner selection
    Object(&'static str, String, &'static str, Vec<FieldSelector>),
    /// field name, arguments, inner selection by type
    Union(
        &'static str,
        String,
        Arc<dyn Fn(&str) -> Vec<FieldSelector>>
    )
}

impl fmt::Debug for FieldSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldSelector::Scalar(field_name, args) => {
                write!(f, "Scalar(field name: {}, args: {})", field_name, args)
            }
            FieldSelector::Object(field_name, args, typename, _) => write!(
                f,
                "Object(field name: {}, args: {}, typename: {})",
                field_name, args, typename
            ),
            FieldSelector::Union(field_name, args, _) => {
                write!(f, "Union(field name: {}, args: {})", field_name, args)
            }
        }
    }
}

/// Metadata for an operation
#[derive(Clone, Debug, PartialEq)]
pub struct OperationMeta {
    /// The query key before being hashed with the variables
    pub query_key: u32,
    /// The type of the operation, query, mutation or subscription
    pub operation_type: OperationType,
    /// A list of types that are returned by the query
    pub involved_types: Vec<&'static str>
}

/// Options for the operation.
/// This is just an internal representation of the union between `QueryOptions` and `ClientOptions`.
#[derive(Clone)]
pub struct OperationOptions {
    /// The url that should be used when fetching
    pub url: String,
    /// Extra headers that should be applied when fetching
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    /// The request policy of the query. Exchanges should respect this.
    pub request_policy: RequestPolicy,
    /// Extensions that can contain extra configuration for exchanges.
    pub extensions: Option<Extensions>,
    /// The fetch function passed by JavaScript code
    #[cfg(target_arch = "wasm32")]
    pub fetch: Option<js_sys::Function>
}

// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl Send for OperationOptions {}
// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl Sync for OperationOptions {}

/// A query operation. One of these is fired for each direct query, as well as for query reruns.
#[derive(Clone)]
pub struct Operation<V: Serialize + Clone + Send + Sync> {
    /// The unique key of the operation. This will identify a unique combination of query and
    /// variables.
    /// This means calling the same query with the same variables will produce the same key,
    /// regardless of where the operation originates
    pub key: u64,
    /// Operation metadata such as the query key
    pub meta: OperationMeta,
    /// The query body. This will be serialized by the fetch implementation but may also be used
    /// to get a reference to the query variables.
    pub query: QueryBody<V>,
    /// The options of the operation. This is an OR union of `QueryOptions` and `ClientOptions`.
    pub options: OperationOptions
}

/// The source of the result (cache or network).
/// Used for debugging.
#[derive(Clone, Debug, PartialEq, Copy, Serialize)]
pub enum ResultSource {
    Cache,
    Network
}

/// Debug info used for... well, debugging.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DebugInfo {
    /// The source of the result, cache or network
    pub source: ResultSource,
    /// Whether the query was actually run (`false`) or combined with another query in a deduplication exchange (`true`)
    #[serde(rename = "didDedup")]
    pub did_dedup: bool
}

/// The result of a successful operation.
#[derive(Clone, Debug, PartialEq)]
pub struct OperationResult<R: DeserializeOwned + Send + Sync + Clone> {
    /// The key of the operation passed back by the last exchange
    pub key: u64,
    /// The metadata of the operation passed back by the last exchange
    pub meta: OperationMeta,
    /// The deserialized response
    pub response: Response<R>
}

/// An observable result. This implements `Stream` and unsubscribes on drop.
/// It will receive early (partial or stale) results, as well as refreshing when the query is
/// rerun after being invalidated by mutations.
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
impl<T: Clone, M: Exchange> Observable<T, M> {
    /// Manually cause the client to rerun this query.
    /// Note this doesn't invalidate any caching, so if the query is in the cache it will simply be re-read
    pub fn rerun(&self) {
        self.client.rerun_query(self.key);
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

/// An extension that may be passed by the user to provide additional request options to a third-party exchange.
/// This is only here to allow for JS interop - The implementor of the exchange must deserialize JavaScript input to the best of their ability.
/// Note that this may involve complex operations such as converting `js_sys::Function` to Rust closures or other advanced deserialization.
/// This is not a simple serde-like deserialization.
///
/// An extension must always be `Clone`.
///
/// # Example
///
/// ```
/// use artemis::exchange::Extension;
///
/// #[derive(Clone)]
/// struct MyExtension(String);
///
/// impl Extension for MyExtension {
///     #[cfg(target_arch = "wasm32")]
///     fn from_js(value: JsValue) -> Option<Self> {
///         use wasm_bindgen::JsCast;
///
///         let cast = value.dyn_into::<String>();
///         if let Ok(cast) = cast {
///             Self(cast)
///         } else { None }
///     }
/// }
/// ```
pub trait Extension: Sized + Clone + Send + Sync + 'static {
    #[cfg(target_arch = "wasm32")]
    fn from_js(value: JsValue) -> Option<Self>;
}

/// A map of keyed extensions.
/// The key is only used for JS interop,
/// the Rust version uses the type as the key.
///
/// This is usually instantiated by the [`ext![]`](./macro.ext!.html) macro.
pub struct ExtensionMap {
    rust: HashMap<TypeId, Box<dyn Any>>,
    #[cfg(target_arch = "wasm32")]
    js: JsValue
}

// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl Send for ExtensionMap {}
// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl Sync for ExtensionMap {}

impl Default for ExtensionMap {
    fn default() -> Self {
        Self {
            rust: HashMap::new(),
            #[cfg(target_arch = "wasm32")]
            js: JsValue::NULL
        }
    }
}

impl ExtensionMap {
    /// Create a new extension map.
    /// This is usually just called by the [`ext![]`](./macro.ext!.html) macro.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new map from a JavaScript value.
    /// This is used internally but must be public due to use in the
    /// [`wasm_client!`](./macro.wasm_client!.html) macro.
    #[cfg(target_arch = "wasm32")]
    pub fn from_js(value: JsValue) -> Option<Self> {
        if value.is_object() {
            Some(Self {
                rust: HashMap::new(),
                js: value
            })
        } else {
            None
        }
    }

    /// Insert a value into the map.
    /// This is usually called by the [`ext![]`](./macro.ext!.html) macro.
    pub fn insert<T: Extension>(&mut self, value: T) {
        self.rust.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Get a value from the map.
    /// The key is only used to get the value out of the JavaScript object,
    /// if the extension was inserted on the Rust side it doesn't do anything.
    pub fn get<T: Extension, S: Into<String>>(&self, js_key: S) -> Option<T> {
        self.get_rust().or_else(|| self.get_js(js_key.into()))
    }

    #[cfg(target_arch = "wasm32")]
    fn get_js<T: Extension>(&self, js_key: String) -> Option<T> {
        let key: JsValue = js_key.into();
        js_sys::Reflect::get(&self.js, &key)
            .ok()
            .and_then(|value| T::from_js(value))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get_js<T: Extension>(&self, _js_key: String) -> Option<T> {
        None
    }

    fn get_rust<T: Extension>(&self) -> Option<T> {
        self.rust
            .get(&TypeId::of::<T>())
            .map(|boxed| (&*boxed).downcast_ref().unwrap())
            .map(Clone::clone)
    }
}

/// A thread-safe wrapper around [ExtensionMap](./struct.ExtensionMap.html).
pub type Extensions = Arc<ExtensionMap>;

/// Options that can be passed to a query.
/// This will be combined with `ClientOptions`, but `QueryOptions` takes precedence.
#[derive(Default, Clone)]
pub struct QueryOptions {
    /// The URL of your GraphQL Endpoint
    pub url: Option<String>,
    /// A function that returns extra headers. This is a function to allow for dynamic creation
    /// of things such as authorization headers
    pub extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    /// The policy to use for this request. See `RequestPolicy`
    pub request_policy: Option<RequestPolicy>,
    /// Extra extensions passed to the exchanges. Allows for configuration of custom exchanges.
    pub extensions: Option<Extensions>
}

/// Client trait passed to exchanges. Only exposes methods useful to exchanges
pub trait Client: Clone + Send + Sync + 'static {
    /// Rerun a query with that key and push the result to all subscribers.
    fn rerun_query(&self, query_key: u64);

    /// Push a new result to any subscribers.
    fn push_result<R>(&self, query_key: u64, result: ExchangeResult<R>)
    where
        R: DeserializeOwned + Send + Sync + Clone + 'static;
}
