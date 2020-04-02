use std::sync::Arc;

mod builder;
mod r#impl;
#[cfg(feature = "observable")]
mod observable;

use crate::{
    exchanges::TerminatorExchange, Exchange, GraphQLQuery, QueryError, QueryOptions, Response
};
pub use builder::ClientBuilder;
pub use r#impl::ClientImpl;
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[derive(Clone)]
#[repr(transparent)]
pub struct Client<M: Exchange = TerminatorExchange>(pub Arc<ClientImpl<M>>);

impl Client {
    /// Returns a `ClientBuilder` with the given endpoint URL
    pub fn builder<U: Into<String>>(url: U) -> ClientBuilder {
        ClientBuilder::new(url)
    }
}

impl<M: Exchange> Client<M> {
    /// Executes a query with the given variables
    /// Returns the result of the query, or a `QueryError` if one of the exchanges encountered a fatal error.
    ///
    /// # Example
    ///
    /// ```
    /// # use artemis_test::get_conference::{GetConference, get_conference::Variables};
    /// # use artemis::{Client, ClientBuilder};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(async {
    /// let client = ClientBuilder::new("http://localhost:8080/graphql")
    ///     .with_default_exchanges()
    ///     .build();
    ///
    /// let result = client.query(GetConference, Variables { id: "1".to_string() }).await.unwrap();
    ///
    /// assert!(result.data.is_some())
    /// # });
    /// ```
    pub async fn query<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        self.0.query(_query, variables).await
    }

    /// Executes a query with the given variables and options
    /// Returns the result of the query, or a `QueryError` if one of the exchanges encountered a fatal error.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use artemis_test::get_conference::{GetConference, get_conference::Variables};
    /// # use artemis::{Client, ClientBuilder};
    /// # use futures::StreamExt;
    ///
    /// let client = ClientBuilder::new("http://localhost:8080/graphql")
    ///     .with_default_exchanges()
    ///     .build();
    ///
    /// let result = client.query(GetConference, Variables { id: "1".to_string() }).await.unwrap();
    ///
    /// assert!(result.data.is_some())
    /// ```
    pub async fn query_with_options<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        self.0.query_with_options(_query, variables, options).await
    }

    /// Subscribes to a query, returning any potential early results, the initial result and any future updates
    /// The function returns an `Observable` which can be subscribed to like a regular stream.
    /// Dropping the `Observable` will cancel the subscription.
    ///
    /// Requires feature: `observable`
    ///
    /// # Example
    ///
    /// ```
    /// # use artemis_test::get_conference::{GetConference, get_conference::Variables};
    /// # use artemis::{Client, ClientBuilder};
    /// # use futures::StreamExt;
    /// # tokio_test::block_on(async {
    /// let client = ClientBuilder::new("http://localhost:8080/graphql")
    ///     .with_default_exchanges()
    ///     .build();
    ///
    /// let mut observable = client.subscribe(GetConference, Variables { id: "1".to_string() });
    /// let result = observable.next().await.unwrap().unwrap();
    ///
    /// assert!(result.data.is_some())
    /// # });
    /// ```
    #[cfg(all(not(target_arch = "wasm32"), feature = "observable"))]
    pub fn subscribe<Q: GraphQLQuery + 'static>(
        &self,
        query: Q,
        variables: Q::Variables
    ) -> observable::OperationObservable<Q, M> {
        self.0.subscribe(query, variables)
    }

    /// Subscribes to a query with options, returning any potential early results, the initial result and any future updates
    /// The function returns an `Observable` which can be subscribed to like a regular stream.
    /// Dropping the `Observable` will cancel the subscription.
    ///
    /// Requires feature: `observable`
    ///
    /// # Example
    /// ```ignore
    /// # use artemis_test::get_conference::{GetConference, get_conference::Variables};
    /// # use artemis::{Client, ClientBuilder};
    /// # use futures::StreamExt;
    ///
    /// let client = ClientBuilder::new("http://localhost:8080/graphql")
    ///     .with_default_exchanges()
    ///     .build();
    ///
    /// let mut observable = client.subscribe(GetConference, Variables { id: "1".to_string() });
    /// let result = observable.next().await.unwrap().unwrap();
    ///
    /// assert!(result.data.is_some())
    /// ```
    #[cfg(all(not(target_arch = "wasm32"), feature = "observable"))]
    pub fn subscribe_with_options<Q: GraphQLQuery + 'static>(
        &self,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> observable::OperationObservable<Q, M> {
        self.0.subscribe_with_options(_query, variables, options)
    }
}
