#![deny(warnings)]
#![allow(unused_parens)]
#![cfg_attr(test, feature(proc_macro_hygiene))]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate lazy_static;

mod cache_exchange;
mod store;
mod types;

pub use store::{QueryStore, Store};
pub use types::{NormalizedCacheExtension, NormalizedCacheOptions};
