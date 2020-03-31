use serde::Serialize;
use std::num::Wrapping;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// When we have separate values it's useful to run a progressive
/// version of djb2 where we pretend that we're still looping over
/// the same value
/// TODO: Figure out why this gives different results on different OS
pub fn progressive_hash<V: Serialize>(h: u32, x: &V) -> u64 {
    let x = bincode::serialize(x).expect("Failed to convert variables to Vec<u8> for hashing");

    let mut h = Wrapping(h as u64);

    for byte in x {
        h = (h << 5) + h + Wrapping(byte as u64)
    }

    h.0
}

#[macro_export]
macro_rules! ext {
    ($($x: expr),*) => {
        {
            let mut typemap = ::artemis::ExtensionMap::new();
            $(
                typemap.insert($x);
            )*
            ::std::sync::Arc::new(typemap)
        }
    };
}

#[cfg(all(target_arch = "wasm32", feature = "observable"))]
pub mod wasm {
    use crate::{
        ClientImpl, DebugInfo, Error, Exchange, ExtensionMap, GraphQLQuery, HeaderPair, QueryError,
        QueryOptions, RequestPolicy, Response
    };
    use futures::{future::BoxFuture, Stream, StreamExt};
    use js_sys::{Array, Function};
    use serde::Serialize;
    use std::{
        any::Any,
        future::Future,
        pin::Pin,
        sync::Arc,
        task::{Context, Poll}
    };
    use wasm_bindgen::{
        closure::Closure, prelude::*, JsCast, JsValue, __rt::std::collections::HashMap
    };

    #[wasm_bindgen(typescript_custom_section)]
    const TS_APPEND_CONTENT: &'static str = r#"
export type Maybe<T> = T | null | undefined;

export type Response<T> = { data: Maybe<T>, errors: Maybe<Error[]>, debugInfo: Maybe<DebugInfo> }

export type Extensions = { [K: string]: any }

export type Error = {
    message: string,
    locations?: Location[],
    path?: PathFragment[],
    extensions?: Extensions
}

export type PathFragment = string | number

export type Location = {
    line: number,
    column: number
}

export type DebugInfo = {
    source: ResultSource,
    didDedup: boolean
}

export type ResultSource = "Cache" | "Network"

export type ClientOptions = {
    url?: string,
    headers?: () => Headers,
    requestPolicy?: RequestPolicy,
    fetch?: (url: string, init: RequestInit) => Promise<any>
};

export type Headers = { [K: string]: string };

export enum RequestPolicy {
    CacheFirst = 1,
    CacheOnly = 2,
    NetworkOnly = 3,
    CacheAndNetwork = 4
}

export type QueryOptions = {
    url?: string,
    headers?: () => Headers,
    requestPolicy?: RequestPolicy,
    extensions?: ExtensionMap
};

export type ExtensionMap = { [K: string]: Extension };

/**
 * This corresponds to the Rust side Extension trait.
 * Any extension class will work here, it's just a semantic type.
 */
export type Extension = any;

export interface ArtemisClient<Q> {
    new (options: ClientOptions): ArtemisClient<Q>,
    query<R = object, V = object>(query: Q, variables: V, options?: QueryOptions): Promise<Response<R>>,
    subscribe<R = object, V = object>(query: Q, variables: V, callback: (ok: Maybe<Response<R>>, err: any) => void, options?: QueryOptions): void,
    free(): void
}
"#;

    unsafe impl Send for JsFunction {}
    unsafe impl Sync for JsFunction {}

    #[derive(Clone)]
    pub struct JsFunction(Function);

    #[wasm_bindgen]
    extern "C" {
        pub type JsClientOptions;

        #[wasm_bindgen(method, getter, structural)]
        pub fn url(this: &JsClientOptions) -> Option<String>;
        #[wasm_bindgen(method, getter, structural)]
        pub fn headers(this: &JsClientOptions) -> Option<Function>;
        #[wasm_bindgen(method, getter = requestPolicy, structural)]
        pub fn request_policy(this: &JsClientOptions) -> Option<u8>;
        #[wasm_bindgen(method, getter, structural)]
        pub fn fetch(this: &JsClientOptions) -> Option<js_sys::Function>;

        pub type JsQueryOptions;

        #[wasm_bindgen(method, getter = url, structural)]
        pub fn url2(this: &JsQueryOptions) -> Option<String>;
        #[wasm_bindgen(method, getter = headers, structural)]
        pub fn headers2(this: &JsQueryOptions) -> Option<Function>;
        #[wasm_bindgen(method, getter = requestPolicy, structural)]
        pub fn request_policy2(this: &JsQueryOptions) -> Option<u8>;
        #[wasm_bindgen(method, getter = extensions, structural)]
        pub fn extensions2(this: &JsQueryOptions) -> JsValue;
    }

    impl From<JsQueryOptions> for QueryOptions {
        fn from(options: JsQueryOptions) -> Self {
            unsafe {
                let extensions = ExtensionMap::from_js(options.extensions2());
                QueryOptions {
                    url: options.url2().map(|url| url.parse().unwrap()),
                    extra_headers: options.headers2().map(convert_header_fn),
                    request_policy: options.request_policy2().map(Into::into),
                    extensions: extensions.map(Arc::new)
                }
            }
        }
    }

    pub struct UnsafeSendFuture<T> {
        fut: Pin<Box<dyn Future<Output = T> + 'static>>
    }

    unsafe impl<T> Send for UnsafeSendFuture<T> {}

    impl<T> UnsafeSendFuture<T> {
        pub fn new(fut: Pin<Box<dyn Future<Output = T> + 'static>>) -> Self {
            Self { fut }
        }
    }

    impl<T> Future for UnsafeSendFuture<T> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            // This is safe because we're only using this future as a pass-through for the inner
            // future, in order to implement `Send`. If it's safe to poll the inner future, it's safe
            // to proxy it too.
            unsafe { Pin::new_unchecked(&mut self.fut).poll(cx) }
        }
    }

    pub trait QueryCollection {
        fn query<E: Exchange>(
            self,
            client: Arc<ClientImpl<E>>,
            variables: JsValue,
            options: QueryOptions
        ) -> BoxFuture<'static, Result<JsValue, JsValue>>;
        fn subscribe<E: Exchange>(
            self,
            client: Arc<ClientImpl<E>>,
            variables: JsValue,
            callback: Function,
            options: QueryOptions
        );
    }

    #[wasm_bindgen(js_name = QueryError)]
    #[derive(Serialize)]
    pub struct JsQueryError {
        message: String
    }

    impl From<QueryError> for JsQueryError {
        fn from(e: QueryError) -> Self {
            JsQueryError {
                message: e.to_string()
            }
        }
    }

    pub fn convert_header_fn(fun: Function) -> Arc<dyn (Fn() -> Vec<HeaderPair>) + Send + Sync> {
        let fun = JsFunction(fun);
        Arc::new(move || {
            let this = JsValue::NULL;
            let result = fun.0.call0(&this).unwrap();
            let map: HashMap<String, String> = serde_wasm_bindgen::from_value(result).unwrap();
            map.into_iter()
                .map(|(key, value)| HeaderPair(key, value))
                .collect()
        })
    }

    pub fn bind_stream<S, Item>(mut stream: S, callback: Function)
    where
        S: Stream<Item = Result<Item, QueryError>> + 'static,
        Item: Serialize + 'static
    {
        let callback = JsFunction(callback);
        let fut = stream.fold((), move |_, next| {
            let callback = callback.clone();
            async move {
                let this = JsValue::NULL;
                let (ok, err) = match next {
                    Ok(value) => (Some(value), None),
                    Err(e) => (None, Some(JsQueryError::from(e)))
                };
                let ok = serde_wasm_bindgen::to_value(&ok).unwrap();
                let err = serde_wasm_bindgen::to_value(&err).unwrap();
                callback.0.call2(&this, &ok, &err);
            }
        });
        wasm_bindgen_futures::spawn_local(fut);
    }
}
