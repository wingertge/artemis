use crate::{
    client::ClientImpl,
    wasm::{JsQueryOptions, QueryCollection},
    Client, ClientBuilder, Exchange, ExchangeFactory, OperationMeta, QueryBody, QueryOptions,
    RequestPolicy
};
use js_sys::Function;
use serde_json::Value;
use std::{marker::PhantomData, sync::Arc};
use wasm_bindgen::JsValue;
use wasm_typescript_definition::TypescriptDefinition;

pub struct JsClient<M: Exchange, Q: QueryCollection> {
    inner: Arc<ClientImpl<M>>,
    _queries: PhantomData<Q>
}

impl<M: Exchange, Q: QueryCollection> JsClient<M, Q> {
    pub fn new(inner: Client<M>) -> Self {
        let inner = inner.0;
        Self {
            inner,
            _queries: PhantomData
        }
    }

    pub async fn query(
        &self,
        query: Q,
        variables: JsValue,
        options: Option<JsQueryOptions>
    ) -> Result<JsValue, JsValue> {
        query
            .query(
                self.inner.clone(),
                variables,
                options
                    .map(Into::into)
                    .unwrap_or_else(|| QueryOptions::default())
            )
            .await
    }

    pub fn subscribe(
        &self,
        query: Q,
        variables: JsValue,
        callback: Function,
        options: Option<JsQueryOptions>
    ) {
        query.subscribe(
            self.inner.clone(),
            variables,
            callback,
            options
                .map(Into::into)
                .unwrap_or_else(|| QueryOptions::default())
        );
    }
}
