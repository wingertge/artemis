use crate::{
    exchanges::Client,
    types::{ExchangeResult, Operation, OperationResult},
    DebugInfo, Exchange, ExchangeFactory, GraphQLQuery, HeaderPair, OperationOptions, QueryBody,
    Response, ResultSource
};
#[cfg(target_arch = "wasm32")]
use futures::future::BoxFuture;
use std::{
    error::Error,
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll}
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug)]
pub enum FetchError {
    #[cfg(not(target_arch = "wasm32"))]
    NetworkError(Box<dyn Error + Send + Sync>),
    #[cfg(target_arch = "wasm32")]
    NotOk(u16, String, String),
    #[cfg(target_arch = "wasm32")]
    DecodeError(std::io::Error),
    #[cfg(not(target_arch = "wasm32"))]
    DecodeError(reqwest::Error),
    #[cfg(target_arch = "wasm32")]
    EncodeError(serde_json::Error)
}
impl Error for FetchError {}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            FetchError::NetworkError(e) => write!(f, "fetch error: {}", e),
            FetchError::DecodeError(e) => write!(f, "decoding error: {}", e),
            #[cfg(target_arch = "wasm32")]
            FetchError::EncodeError(e) => write!(f, "encoding error: {}", e),
            #[cfg(target_arch = "wasm32")]
            FetchError::NotOk(status_code, status_text, body) => write!(
                f,
                "server returned error code: {} {}\n{}",
                status_code, status_text, body
            )
        }
    }
}

/// The default fetch exchange
///
/// Uses `reqwest` on x86.
/// On `wasm32` it defaults to `window.fetch`,
/// but will use the passed in fetch function if it's set instead
pub struct FetchExchange;

impl<TNext: Exchange> ExchangeFactory<TNext> for FetchExchange {
    type Output = FetchExchange;

    fn build(self, _next: TNext) -> Self::Output {
        FetchExchange
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    pub type JsResponse;

    #[wasm_bindgen(catch, method, structural, js_name = arrayBuffer)]
    pub fn array_buffer(this: &JsResponse) -> Result<::js_sys::Promise, JsValue>;
    #[wasm_bindgen(method, structural, getter)]
    pub fn status(this: &JsResponse) -> u16;
    #[wasm_bindgen(method, structural, getter)]
    pub fn status_text(this: &JsResponse) -> String;
    #[wasm_bindgen(method, structural, getter)]
    pub fn ok(this: &JsResponse) -> bool;
}

impl FetchExchange {
    #[cfg(not(target_arch = "wasm32"))]
    async fn fetch<Q: GraphQLQuery>(
        extra_headers: Vec<HeaderPair>,
        options: OperationOptions,
        query: QueryBody<Q::Variables>
    ) -> Result<Response<Q::ResponseData>, FetchError> {
        let client = reqwest::Client::new();
        let mut request = client
            .post(&options.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&query);

        for HeaderPair(key, value) in extra_headers {
            request = request.header(&key, &value);
        }

        Ok(request
            .send()
            .await
            .map_err(|e| FetchError::NetworkError(Box::new(e)))?
            .json()
            .await
            .map_err(FetchError::DecodeError)?)
    }

    #[cfg(target_arch = "wasm32")]
    fn fetch<Q: GraphQLQuery>(
        extra_headers: Vec<HeaderPair>,
        options: OperationOptions,
        query: QueryBody<Q::Variables>
    ) -> BoxFuture<'static, Result<Response<Q::ResponseData>, FetchError>> {
        use wasm_bindgen::{prelude::*, JsCast};
        use wasm_bindgen_futures::JsFuture;
        use web_sys::RequestMode;

        let fut = async move {
            let url = format!("{}", options.url);
            let body = serde_json::to_string(&query).map_err(FetchError::EncodeError)?;
            let mut init = web_sys::RequestInit::new();
            init.method("POST");

            let headers = web_sys::Headers::new().unwrap();
            for HeaderPair(key, value) in extra_headers {
                headers.set(key.as_str(), value.as_str()).unwrap();
            }
            let headers = headers.into();
            init.headers(&headers);

            init.mode(RequestMode::Cors);
            init.body(Some(&JsValue::from(&body)));

            let promise: js_sys::Promise = if let Some(fetch) = options.fetch {
                let this = JsValue::NULL;
                let url = url.into();
                let init = init.into();

                let promise = fetch.call2(&this, &url, &init).unwrap();
                promise.dyn_into().unwrap()
            } else {
                let window = web_sys::window().expect("A global window object could not be found");
                let request = web_sys::Request::new_with_str_and_init(&url, &init).unwrap();
                window.fetch_with_request(&request)
            };
            let resp = JsFuture::from(promise).await.unwrap();
            let res: JsResponse = resp.unchecked_into();

            let promise = res.array_buffer().unwrap();
            let resp = JsFuture::from(promise).await.unwrap();
            let buf: js_sys::ArrayBuffer = resp.dyn_into().unwrap();
            let slice = js_sys::Uint8Array::new(&buf);
            let mut body: Vec<u8> = vec![0; slice.length() as usize];
            slice.copy_to(&mut body);

            if !res.ok() {
                let body = String::from_utf8(body).unwrap();
                return Err(FetchError::NotOk(res.status(), res.status_text(), body));
            }

            serde_json::from_slice(&body)
                .map_err(std::io::Error::from)
                .map_err(FetchError::DecodeError)
        };

        Box::pin(InnerFuture::<Q> { fut: Box::pin(fut) })
    }
}

// This type e
#[allow(clippy::type_complexity)]
struct InnerFuture<Q: GraphQLQuery> {
    fut: Pin<Box<dyn Future<Output = Result<Response<Q::ResponseData>, FetchError>> + 'static>>
}

// This is safe because WASM doesn't have threads yet. Once WASM supports threads we should use a
// thread to park the blocking implementation until it's been completed.
unsafe impl<Q: GraphQLQuery> Send for InnerFuture<Q> {}

impl<Q: GraphQLQuery> Future for InnerFuture<Q> {
    type Output = Result<Response<Q::ResponseData>, FetchError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // This is safe because we're only using this future as a pass-through for the inner
        // future, in order to implement `Send`. If it's safe to poll the inner future, it's safe
        // to proxy it too.
        unsafe { Pin::new_unchecked(&mut self.fut).poll(cx) }
    }
}

#[async_trait]
impl Exchange for FetchExchange {
    async fn run<Q: GraphQLQuery, C: Client>(
        &self,
        operation: Operation<Q::Variables>,
        _client: C
    ) -> ExchangeResult<Q::ResponseData> {
        let extra_headers = if let Some(ref extra_headers) = operation.options.extra_headers {
            extra_headers()
        } else {
            Vec::new()
        };

        let mut response =
            FetchExchange::fetch::<Q>(extra_headers, operation.options, operation.query).await?;

        let debug_info = Some(DebugInfo {
            // TODO: Make this conditional
            source: ResultSource::Network,
            did_dedup: false
        });

        response.debug_info = debug_info;

        Ok(OperationResult {
            key: operation.key,
            meta: operation.meta,
            response
        })
    }
}
