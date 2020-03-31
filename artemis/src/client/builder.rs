#[cfg(feature = "default-exchanges")]
use crate::exchanges::{CacheExchange, DedupExchange, FetchExchange};
use crate::{
    client::ClientImpl, exchanges::DummyExchange, Client, Exchange, ExchangeFactory, HeaderPair,
    RequestPolicy, Url
};
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct ClientBuilder<M: Exchange = DummyExchange> {
    exchange: M,
    url: Url,
    extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    request_policy: RequestPolicy,
    #[cfg(target_arch = "wasm32")]
    fetch: Option<js_sys::Function>
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl ClientBuilder<DummyExchange> {
    //#[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new<U: Into<String>>(url: U) -> Self {
        let url = url
            .into()
            .parse()
            .expect("Failed to parse url for Artemis client");
        ClientBuilder {
            exchange: DummyExchange,
            url,
            extra_headers: None,
            request_policy: RequestPolicy::CacheFirst,
            #[cfg(target_arch = "wasm32")]
            fetch: None
        }
    }
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl<M: Exchange> ClientBuilder<M> {
    /// Add the default exchanges to the chain. Keep in mind that exchanges are executed bottom to top, so the first one added will be the last one executed.
    #[cfg(feature = "default-exchanges")]
    pub fn with_default_exchanges(self) -> ClientBuilder<impl Exchange> {
        self.with_exchange(FetchExchange)
            .with_exchange(CacheExchange)
            .with_exchange(DedupExchange)
    }

    /// Add a middleware to the chain.
    /// Keep in mind that exchanges are executed bottom to top,
    /// so the first one added will be the last one executed.
    pub fn with_exchange<TResult, F>(self, exchange_factory: F) -> ClientBuilder<TResult>
    where
        TResult: Exchange + Send + Sync,
        F: ExchangeFactory<M, Output = TResult>
    {
        let exchange = exchange_factory.build(self.exchange);
        ClientBuilder {
            exchange,
            url: self.url,
            extra_headers: self.extra_headers,
            request_policy: self.request_policy,
            #[cfg(target_arch = "wasm32")]
            fetch: self.fetch
        }
    }

    pub fn with_extra_headers(
        mut self,
        header_fn: impl Fn() -> Vec<HeaderPair> + Send + Sync + 'static
    ) -> Self {
        self.extra_headers = Some(Arc::new(header_fn));
        self
    }

    #[cfg(target_arch = "wasm32")]
    pub fn with_js_extra_headers(mut self, header_fn: js_sys::Function) -> Self {
        self.extra_headers = Some(crate::wasm::convert_header_fn(header_fn));
        self
    }

    #[cfg(target_arch = "wasm32")]
    pub fn with_fetch(mut self, fetch: js_sys::Function) -> Self {
        self.fetch = Some(fetch);
        self
    }

    pub fn with_request_policy(mut self, request_policy: RequestPolicy) -> Self {
        self.request_policy = request_policy;
        self
    }

    pub fn build(self) -> Client<M> {
        let client = ClientImpl {
            url: self.url,
            exchange: self.exchange,
            extra_headers: self.extra_headers,
            request_policy: self.request_policy,
            active_subscriptions: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(target_arch = "wasm32")]
            fetch: self.fetch
        };

        Client(Arc::new(client))
    }
}
