use crate::{Exchange, Url, HeaderPair, RequestPolicy, exchanges::DummyExchange, ExchangeFactory, Client};
#[cfg(feature = "default-exchanges")]
use crate::exchanges::{CacheExchange, DedupExchange, FetchExchange};
use std::sync::Arc;
use crate::client::ClientImpl;
use parking_lot::Mutex;
use std::collections::HashMap;

pub struct ClientBuilder<M: Exchange = DummyExchange> {
    exchange: M,
    url: Url,
    extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    request_policy: RequestPolicy
}

impl ClientBuilder<DummyExchange> {
    pub fn new<U: Into<String>>(url: U) -> Self {
        let url = url
            .into()
            .parse()
            .expect("Failed to parse url for Artemis client");
        ClientBuilder {
            exchange: DummyExchange,
            url,
            extra_headers: None,
            request_policy: RequestPolicy::CacheFirst
        }
    }
}

impl<M: Exchange> ClientBuilder<M> {
    /// Add the default exchanges to the chain. Keep in mind that exchanges are executed bottom to top, so the first one added will be the last one executed.
    #[cfg(feature = "default-exchanges")]
    pub fn with_default_exchanges(self) -> ClientBuilder<impl Exchange> {
        self.with_exchange(FetchExchange)
            .with_exchange(CacheExchange)
            .with_exchange(DedupExchange)
    }

    /// Add a middleware to the chain. Keep in mind that exchanges are executed bottom to top, so the first one added will be the last one executed.
    pub fn with_exchange<TResult, F>(self, exchange_factory: F) -> ClientBuilder<TResult>
        where
            TResult: Exchange + Send + Sync,
            F: ExchangeFactory<TResult, M>
    {
        let exchange = exchange_factory.build(self.exchange);
        ClientBuilder {
            exchange,
            url: self.url,
            extra_headers: self.extra_headers,
            request_policy: self.request_policy
        }
    }

    pub fn with_extra_headers<F: Fn() -> Vec<HeaderPair> + Send + Sync + 'static>(
        mut self,
        header_fn: F
    ) -> Self {
        self.extra_headers = Some(Arc::new(header_fn));
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
            active_subscriptions: Arc::new(Mutex::new(HashMap::new()))
        };

        Client(Arc::new(client))
    }
}