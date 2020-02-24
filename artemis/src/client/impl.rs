use crate::{
    Exchange, GraphQLQuery, HeaderPair, Operation, OperationMeta, QueryBody, QueryError,
    QueryOptions, RequestPolicy, Response, Url
};
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc, vec};

#[cfg(feature = "observable")]
use crate::client::observable::Subscription;
use crate::types::OperationOptions;

pub struct ClientImpl<M: Exchange> {
    pub(crate) url: Url,
    pub(crate) exchange: M,
    pub(crate) extra_headers: Option<Arc<dyn Fn() -> Vec<HeaderPair> + Send + Sync>>,
    pub(crate) request_policy: RequestPolicy,
    #[cfg(feature = "observable")]
    pub(crate) active_subscriptions: Arc<Mutex<HashMap<u64, Subscription>>>
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
            .run::<Q, M>(operation, self.clone())
            .await
            .map(|operation_result| operation_result.response)
    }

    pub async fn query<Q: GraphQLQuery>(
        self: &Arc<Self>,
        _query: Q,
        variables: Q::Variables
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        let (query, meta) = Q::build_query(variables);
        let operation = self.create_request_operation::<Q>(query, meta, QueryOptions::default());
        self.execute_request_operation::<Q>(operation).await
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
    pub fn rerun_query(self: &Arc<Self>, id: u64) {
        super::observable::rerun_query(self, id);
    }

    #[cfg(not(feature = "observable"))]
    pub fn rerun_query(self: &Arc<Self>, id: u64) {}

    #[cfg(feature = "observable")]
    pub async fn subscribe<Q: GraphQLQuery + 'static>(
        self: &Arc<Self>,
        query: Q,
        variables: Q::Variables
    ) -> super::observable::OperationObservable<Q, M> {
        self.subscribe_with_options(query, variables, QueryOptions::default())
            .await
    }

    #[cfg(feature = "observable")]
    pub async fn subscribe_with_options<Q: GraphQLQuery + 'static>(
        self: &Arc<Self>,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> super::observable::OperationObservable<Q, M> {
        super::observable::subscribe_with_options(self, _query, variables, options).await
    }

    pub(crate) fn create_request_operation<Q: GraphQLQuery>(
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
            query,
            options: OperationOptions {
                url: options.url.unwrap_or_else(|| self.url.clone()),
                extra_headers,
                request_policy: options
                    .request_policy
                    .unwrap_or_else(|| self.request_policy.clone()),
                extensions: options.extensions
            }
        };
        operation
    }
}
