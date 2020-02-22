use crate::{exchanges::{CacheExchange, DedupExchange, DummyExchange, FetchExchange}, types::{Exchange, ExchangeFactory, HeaderPair, Operation, OperationMeta, RequestPolicy}, GraphQLQuery, QueryBody, Response, QueryError};
use std::{sync::Arc};
use surf::url::Url;
use crate::types::Observable;
use parking_lot::Mutex;
use std::collections::HashMap;
use futures::channel::mpsc::Sender;
use std::any::Any;
use futures::{SinkExt, executor};
use std::future::Future;
use std::pin::Pin;

type OperationObservable<Q, M> = Observable<Result<Response<<Q as GraphQLQuery>::ResponseData>, QueryError>, M>;

struct Subscription {
    listeners: Vec<Sender<Arc<dyn Any + Send + Sync>>>,
    // This captures the type and variables of the query without requiring generics, so we can store it in a hashmap
    rerun: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Arc<dyn Any + Send + Sync>> + Send>> + Send + Sync>
}

pub struct ClientBuilder<M: Exchange> {
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
        Client {
            url: self.url,
            exchange: self.exchange,
            extra_headers: self.extra_headers,
            request_policy: self.request_policy,
            active_subscriptions: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

#[derive(Default, Clone)]
pub struct QueryOptions {
    url: Option<Url>,
    extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    request_policy: Option<RequestPolicy>
}

pub struct Client<M: Exchange> {
    url: Url,
    exchange: M,
    extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    request_policy: RequestPolicy,
    active_subscriptions: Arc<Mutex<HashMap<u64, Subscription>>>
}

impl<M: Exchange> Client<M> {
    pub(crate) fn clear_observable(&self, key: u64, index: usize) {
        let mut subscriptions = self.active_subscriptions.lock();
        if let Some(subscription) = subscriptions.get_mut(&key) {
            subscription.listeners.remove(index);
            if subscription.listeners.len() == 0 {
                subscriptions.remove(&key);
            }
        }
    }

    async fn execute_request_operation<Q: GraphQLQuery>(
        &self,
        operation: Operation<Q::Variables>
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        self.exchange.run::<Q>(operation).await
            .map(|operation_result| operation_result.response)
    }

    pub async fn query<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        let (query, meta) = Q::build_query(variables);
        let operation = self.create_request_operation::<Q>(query, meta, QueryOptions::default());
        self.execute_request_operation::<Q>(operation).await
    }

    pub async fn query_with_options<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        let (query, meta) = Q::build_query(variables);
        let operation = self.create_request_operation::<Q>(query, meta, options);
        self.execute_request_operation::<Q>(operation).await
    }

    pub fn rerun_query(self: Arc<Self>, id: u64) {
        let client = self.clone();
        let fut = async move {
            let subscriptions = client.active_subscriptions.clone();
            let mut subscriptions = subscriptions.lock();
            let subscription = subscriptions.get_mut(&id);

            if let Some(Subscription { rerun, listeners }) = subscription {
                let value: Arc<dyn Any + Send + Sync> = executor::block_on(rerun()); //TODO: Work out a better way
                for listener in listeners {
                    executor::block_on(listener.send(value.clone())).unwrap();
                }
            }
        };
        tokio::spawn(fut);
    }

    pub async fn subscribe<Q: GraphQLQuery + 'static>(self: Arc<Self>, query: Q, variables: Q::Variables) -> OperationObservable<Q, M> {
        self.subscribe_with_options(query, variables, QueryOptions::default()).await
    }

    pub async fn subscribe_with_options<Q: GraphQLQuery + 'static>(self: Arc<Self>, _query: Q, variables: Q::Variables, options: QueryOptions) -> OperationObservable<Q, M> {
        let (query, meta) = Q::build_query(variables.clone());
        let (mut sender, receiver) = futures::channel::mpsc::channel(8);
        let key = meta.key.clone();

        let operation = self.create_request_operation::<Q>(query, meta, options.clone());

        let observable = {
            let mut subscriptions = self.active_subscriptions.lock();
            let index = if let Some(subscription) = subscriptions.get_mut(&key) {
                subscription.listeners.push(sender.clone());
                subscription.listeners.len() - 1
            } else {
                let client = self.clone();
                let operation = operation.clone();
                let subscription = Subscription {
                    listeners: vec![sender.clone()],
                    rerun: Arc::new(move || {
                        let client = client.clone();
                        let operation = operation.clone();

                        Box::pin(async move {
                            let res = client.execute_request_operation::<Q>(operation).await;
                            let res_boxed: Arc<dyn Any + Send + Sync> = Arc::new(res);
                            res_boxed
                        })
                    })
                };
                subscriptions.insert(key.clone(), subscription);
                0
            };
            Observable::new(key, receiver, self.clone(), index)
        };

        let res = self.execute_request_operation::<Q>(operation).await;
        sender.send(Arc::new(Box::new(res))).await.unwrap();
        observable
    }

    fn create_request_operation<Q: GraphQLQuery>(
        &self,
        query: QueryBody<Q::Variables>,
        meta: OperationMeta,
        options: QueryOptions
    ) -> Operation<Q::Variables> {
        let extra_headers = if let Some(extra_headers) = options.extra_headers {
            Some(extra_headers.clone())
        } else if let Some(ref extra_headers) = self.extra_headers {
            Some(extra_headers.clone())
        } else {
            None
        };

        let operation = Operation {
            meta,
            url: options.url.unwrap_or_else(|| self.url.clone()),
            extra_headers,
            request_policy: options
                .request_policy
                .unwrap_or_else(|| self.request_policy.clone()),
            query
        };
        operation
    }
}
