#![deny(warnings)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate lazy_static;

mod cache_exchange;
mod store;
mod types;

pub use store::Store;
pub use types::{NormalizedCacheExtension, NormalizedCacheOptions};
