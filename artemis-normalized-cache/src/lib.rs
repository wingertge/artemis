//! This is a normalized cache exchange for the [`artemis`](../artemis/index.html) GraphQL Client.
//! This is a drop-in replacement for the default [`CacheExchange`] that, instead of document
//! caching, caches normalized data by keys and connections between data.
//!
//! `artemis` is already quite a comprehensive GraphQL client. However in several cases it may be
//! desirable to have data update across the entirety of an app when a response updates some known
//! pieces of data.
//!
//! # Quick Start
//!
//! After installing this crate, change the default `artemis` Client like from something like this:
//!
//! ```
//! let client = artemis::ClientBuilder::new("http://0.0.0.0")
//!     .with_default_exchanges()
//!     .build();
//! ```
//!
//! to this
//!
//! ```
//! use artemis::default_exchanges::{FetchExchange, DedupExchange};
//! use artemis_normalized_cache::NormalizedCacheExchange;
//!
//! let client = artemis::ClientBuilder::new("http://0.0.0.0")
//!     .with_exchange(FetchExchange)
//!     .with_exchange(NormalizedCacheExchange::new())
//!     .with_exchange(DedupExchange)
//!     .build();
//! ```
//!
//! TODO: Don't steal urlq's docs you plagiarist

#![deny(missing_docs)]
#![allow(unused_parens)]
#![cfg_attr(test, feature(proc_macro_hygiene))]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate lazy_static;

pub mod cache_exchange;
mod store;
mod types;

pub use cache_exchange::NormalizedCacheExchange;
pub use store::QueryStore;
pub use types::{NormalizedCacheExtension, NormalizedCacheOptions};
