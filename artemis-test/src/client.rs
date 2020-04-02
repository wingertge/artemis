use crate::queries::wasm::Queries;
use artemis::{
    default_exchanges::{CacheExchange, DedupExchange, FetchExchange},
    wasm_client, RequestPolicy
};
use wasm_bindgen::prelude::*;

wasm_client! {
    exchanges: [
        FetchExchange,
        CacheExchange,
        DedupExchange
    ],
    request_policy: RequestPolicy::CacheFirst,
    queries: Queries
}
