use std::sync::Arc;

mod builder;
mod r#impl;
#[cfg(feature = "observable")]
mod observable;

use crate::{exchanges::DummyExchange, Exchange, GraphQLQuery, QueryError, QueryOptions, Response};
pub use builder::ClientBuilder;
use futures::{TryFutureExt, TryStreamExt};
pub use r#impl::ClientImpl;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[derive(Clone)]
#[repr(transparent)]
//#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Client<M: Exchange = DummyExchange>(pub Arc<ClientImpl<M>>);

impl Client {
    pub fn builder<U: Into<String>>(url: U) -> ClientBuilder {
        ClientBuilder::new(url)
    }
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl<M: Exchange> Client<M> {
    pub async fn query<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        self.0.query(_query, variables).await
    }

    pub async fn query_with_options<Q: GraphQLQuery>(
        &self,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> Result<Response<Q::ResponseData>, QueryError> {
        self.0.query_with_options(_query, variables, options).await
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "observable"))]
    pub async fn subscribe<Q: GraphQLQuery + 'static>(
        &self,
        query: Q,
        variables: Q::Variables
    ) -> observable::OperationObservable<Q, M> {
        self.0.subscribe(query, variables).await
    }

    /*    #[cfg(all(target_arch = "wasm32", feature = "observable"))]
    pub async fn subscribe<Q: GraphQLQuery + 'static>(
        &self,
        query: Q,
        variables: Q::Variables,
        callback: js_sys::Function
    ) where Q::ResponseData: Into<wasm_bindgen::JsValue> + Unpin {
        use crate::utils::wasm::JsResponse;
        let observable = self.0.subscribe(query, variables).await
            .map_ok(Into::into)
            .map_err(Into::into);
        crate::utils::wasm::bind_stream::<_, JsResponse>(observable, callback);
    }*/

    #[cfg(all(not(target_arch = "wasm32"), feature = "observable"))]
    pub async fn subscribe_with_options<Q: GraphQLQuery + 'static>(
        &self,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions
    ) -> observable::OperationObservable<Q, M> {
        self.0
            .subscribe_with_options(_query, variables, options)
            .await
    }

    /*    #[cfg(all(target_arch = "wasm32", feature = "observable"))]
    pub async fn subscribe_with_options<Q: GraphQLQuery + 'static>(
        &self,
        _query: Q,
        variables: Q::Variables,
        options: QueryOptions,
        callback: js_sys::Function
    ) where Q::ResponseData: Into<wasm_bindgen::JsValue> + Unpin {
        use crate::utils::wasm::JsResponse;
        let observable = self.0
            .subscribe_with_options(_query, variables, options)
            .await
            .map_ok(Into::into)
            .map_err(Into::into);
        crate::utils::wasm::bind_stream::<_, JsResponse>(observable, callback);
    }*/
}
