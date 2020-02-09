use crate::{
    middlewares::{DummyMiddleware, FetchMiddleware},
    types::{HeaderPair, Middleware, MiddlewareFactory, Operation, OperationMeta, RequestPolicy},
    utils::progressive_hash,
    GraphQLQuery, QueryBody, Response
};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, sync::Arc};
use surf::url::Url;
use crate::middlewares::CacheMiddleware;

pub struct ClientBuilder<M>
where
    M: Middleware + Send + Sync
{
    middleware: M,
    url: Url,
    extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    request_policy: RequestPolicy
}

impl ClientBuilder<DummyMiddleware> {
    pub fn new<U: Into<String>>(url: U) -> Self {
        let url = url
            .into()
            .parse()
            .expect("Failed to parse url for Artemis client");
        ClientBuilder {
            middleware: DummyMiddleware,
            url,
            extra_headers: None,
            request_policy: RequestPolicy::CacheFirst
        }
    }
}

impl<M> ClientBuilder<M>
where
    M: Middleware + Send + Sync
{
    /// Add the default middlewares to the chain. Keep in mind that middlewares are executed bottom to top, so the first one added will be the last one executed.
    pub fn with_default_middleware(self) -> ClientBuilder<CacheMiddleware<FetchMiddleware>> {
        let middleware = self.middleware;
        let middleware = FetchMiddleware::build(middleware);
        let middleware = CacheMiddleware::build(middleware);
        ClientBuilder {
            middleware,
            url: self.url,
            extra_headers: self.extra_headers,
            request_policy: self.request_policy
        }
    }

    /// Add a middleware to the chain. Keep in mind that middlewares are executed bottom to top, so the first one added will be the last one executed.
    pub fn with_middleware<TResult, F>(self, _middleware_factory: F) -> ClientBuilder<TResult>
    where
        TResult: Middleware + Send + Sync,
        F: MiddlewareFactory<TResult, M>
    {
        let middleware = F::build(self.middleware);
        ClientBuilder {
            middleware,
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
            middleware: self.middleware,
            extra_headers: self.extra_headers,
            request_policy: self.request_policy
        }
    }
}

#[derive(Default)]
pub struct QueryOptions {
    url: Option<Url>,
    extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    request_policy: Option<RequestPolicy>
}

pub struct Client<M>
where
    M: Middleware + Send + Sync
{
    url: Url,
    middleware: M,
    extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    request_policy: RequestPolicy
}

impl<M> Client<M>
where
    M: Middleware + Send + Sync
{
    async fn execute_request_operation<TVariables, TResult>(
        &self,
        operation: Operation<TVariables>
    ) -> Result<Response<TResult>, Box<dyn Error>>
    where
        TVariables: Serialize + Send + Sync,
        TResult: DeserializeOwned + Send + Sync
    {
        let operation_result = self.middleware.run(operation).await?;
        Ok(serde_json::from_str(
            operation_result.response_string.as_str()
        )?)
    }

    pub async fn query<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables
    ) -> Result<Response<Q::ResponseData>, Box<dyn Error>> {
        let (query, meta) = Q::build_query(variables);
        let operation = self.create_request_operation(query, meta, QueryOptions::default());
        self.execute_request_operation(operation).await
    }

    pub async fn query_with_options<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> Result<Response<Q::ResponseData>, Box<dyn Error>> {
        let (query, meta) = Q::build_query(variables);
        let operation = self.create_request_operation(query, meta, options);
        self.execute_request_operation(operation).await
    }

    fn create_request_operation<V: Serialize + Send + Sync>(
        &self,
        query: QueryBody<V>,
        meta: OperationMeta,
        options: QueryOptions
    ) -> Operation<V> {
        let extra_headers = if let Some(extra_headers) = options.extra_headers {
            Some(extra_headers.clone())
        } else if let Some(ref extra_headers) = self.extra_headers {
            Some(extra_headers.clone())
        } else {
            None
        };

        let key = progressive_hash(&meta.key, &query.variables);
        let meta = OperationMeta { key, ..meta };

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
