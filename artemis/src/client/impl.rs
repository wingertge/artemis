use crate::{
    progressive_hash, Exchange, ExchangeResult, GraphQLQuery, HeaderPair, Operation, OperationMeta,
    QueryBody, QueryError, QueryOptions, RequestPolicy, Response
};
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "observable")]
use crate::client::observable::Subscription;
use crate::types::OperationOptions;
use serde::de::DeserializeOwned;

// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl<M: Exchange> Send for ClientImpl<M> {}
// SAFETY: JavaScript doesn't have multi-threading
// The only non-send value is the pointer in JsValue
unsafe impl<M: Exchange> Sync for ClientImpl<M> {}

pub struct ClientImpl<M: Exchange> {
    pub(crate) url: String,
    pub(crate) exchange: M,
    pub(crate) extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    pub(crate) request_policy: RequestPolicy,
    #[cfg(feature = "observable")]
    pub(crate) active_subscriptions: Arc<Mutex<HashMap<u64, Subscription>>>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) fetch: Option<js_sys::Function>
}

impl<M: Exchange> crate::exchanges::Client for Arc<ClientImpl<M>> {
    fn rerun_query(&self, query_key: u64) {
        if cfg!(feature = "observable") {
            super::observable::rerun_query(self, query_key);
        }
    }

    fn push_result<R>(&self, key: u64, result: ExchangeResult<R>)
    where
        R: DeserializeOwned + Send + Sync + Clone + 'static
    {
        if cfg!(feature = "observable") {
            super::observable::push_result(self, key, result);
        }
    }
}

impl<M: Exchange> ClientImpl<M> {
    #[cfg(feature = "observable")]
    pub(crate) fn clear_observable(&self, key: u64, index: usize) {
        let mut subscriptions = self.active_subscriptions.lock();
        if let Some(subscription) = subscriptions.get_mut(&key) {
            subscription.listeners.remove(index);
            if subscription.listeners.is_empty() {
                subscriptions.remove(&key);
            }
        }
    }

    pub(crate) async fn execute_request_operation<Q: GraphQLQuery>(
        self: &Arc<Self>,
        operation: Operation<Q::Variables>
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        self.exchange
            .run::<Q, _>(operation, self.clone())
            .await
            .map(|operation_result| operation_result.response)
    }

    pub async fn query<Q: GraphQLQuery>(
        self: &Arc<Self>,
        _query: Q,
        variables: Q::Variables
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        self.query_with_options(_query, variables, QueryOptions::default())
            .await
    }

    pub async fn query_with_options<Q: GraphQLQuery>(
        self: &Arc<Self>,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        let (query, meta) = Q::build_query(variables);
        let operation = self.create_request_operation::<Q>(query, meta, options);
        self.execute_request_operation::<Q>(operation).await
    }

    #[cfg(feature = "observable")]
    pub fn subscribe<Q: GraphQLQuery + 'static>(
        self: &Arc<Self>,
        query: Q,
        variables: Q::Variables
    ) -> super::observable::OperationObservable<Q, M> {
        self.subscribe_with_options(query, variables, QueryOptions::default())
    }

    #[cfg(feature = "observable")]
    pub fn subscribe_with_options<Q: GraphQLQuery + 'static>(
        self: &Arc<Self>,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> super::observable::OperationObservable<Q, M> {
        super::observable::subscribe_with_options(self, _query, variables, options)
    }

    pub(crate) fn create_request_operation<Q: GraphQLQuery>(
        &self,
        query: QueryBody<Q::Variables>,
        meta: OperationMeta,
        options: QueryOptions
    ) -> Operation<Q::Variables> {
        let extra_headers = if let Some(extra_headers) = options.extra_headers {
            Some(extra_headers)
        } else if let Some(ref extra_headers) = self.extra_headers {
            Some(extra_headers.clone())
        } else {
            None
        };

        let key = progressive_hash(meta.query_key, &query.variables);

        Operation {
            key,
            meta,
            query,
            options: OperationOptions {
                url: options.url.unwrap_or_else(|| self.url.clone()),
                extra_headers,
                request_policy: options
                    .request_policy
                    .unwrap_or_else(|| self.request_policy.clone()),
                extensions: options.extensions,
                #[cfg(target_arch = "wasm32")]
                fetch: self.fetch.clone()
            }
        }
    }
}
