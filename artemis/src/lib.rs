//! A modern GraphQL Client with common built-in features
//! as well as the ability to extend its functionality through exchanges
//!
//! # Getting Started
//!
//! The first step is to write some queries in `.graphql` files and then add the following to your
//! `build.rs` (create it if necessary):
//!
//! ```ignore
//! use artemis_build::CodegenBuilder;
//!
//! fn main() {
//!     CodegenBuilder::new()
//!         .introspect_schema("http://localhost:8080/graphql", None, Vec::new())
//!         .unwrap()
//!         .add_query("queries/x.graphql")
//!         .with_out_dir("src/queries")
//!         .build()
//!         .unwrap();
//! }
//! ```
//!
//! Afterwards, you can use the crate in your application as such:
//!
//! ```
//! # tokio_test::block_on(async {
//! use artemis::Client;
//! use artemis_test::get_conference::{GetConference, get_conference::Variables};
//!
//! let client = Client::builder("http://localhost:8080/graphql")
//!     .with_default_exchanges()
//!     .build();
//!
//! let result = client.query(GetConference, Variables { id: "1".to_string() }).await.unwrap();
//! assert!(result.data.is_some());
//! # });
//! ```
//!
//! For more info see the relevant method and struct documentation.
//!
//! # Build
//!
//! This crate uses code generation to take your GraphQL files and turn them into
//! strongly typed Rust modules. These contain the query struct, a zero-size type
//! such as `GetConference`, as well as a submodule containing the `Variables`,
//! any input types, the `ResponseData` type and any involved output types.
//!
//! Having a strongly typed compile time representation with additional info
//! (such as the `__typename` of all involved types and an abstract selection tree)
//! means that the work the CPU has to do at runtime is very minimal,
//! only amounting to serialization, deserialization and simple lookups using
//! the statically generated data.
//!
//! For details on how to use the query builder, see [artemis-build](../artemis_build/index.html)
//!
//! # Exchanges
//!
//! Exchanges are like a bi-directional middleware.
//! They act on both the incoming and outgoing queries,
//! passing them on if they can't return a result themselves.
//!
//! There are three default exchanges, called in this order:
//!
//! ## DedupExchange
//!
//! The deduplication exchange (`DedupExchange`) filters out unnecessary queries
//! by combining multiple identical queries into one. It does so by keeping track
//! of in-flight queries and, instead of firing off another identical query,
//! waiting for their results instead. This reduces network traffic,
//! especially in larger applications where the same query may be used in multiple
//! places and run multiple times simultaneously as a result.
//!
//! ## CacheExchange
//!
//! The cache exchange is a very basic, un-normalized cache which eagerly invalidates queries.
//! It's focused on simplicity and correctness of data, so if a query uses any of the same types
//! as a mutation it will always be invalidated by it. This means that especially if you
//! have large amounts of different entities of the same type, this can become expensive quickly.
//! For a more advanced normalized cache that invalidates only directly related entities
//! see the `artemis-normalized-cache` crate.
//!
//! ## FetchExchange
//!
//! The fetch exchange will serialize the query, send it over the network and deserialize the response.
//! This works on x86 using `reqwest`, or `fetch` if you're using WASM.
//! This should be your last exchange in the chain, as it never forwards a query.
//!
//! # WASM
//!
//! WASM support requires some minor boilerplate in your code.
//! First, there's a `wasm` module in your queries. this contains an automatically generated enum
//! containing all your queries. This is used for transmitting type data across the WASM
//! boundary.
//!
//! Second, you have to use the [graphql_client! macro](../artemis_codegen_proc_macro/macro.wasm_client!.html)
//! to generate a WASM interop client that has hard-coded types for your queries, again, to
//! eliminate the unsupported generics and transmit type data across the boundary.
//! The queries type passed to the macro must be the enum generated as mentioned above.
//!
//! Documentation of the JavaScript types and methods can be found in the TypeScript
//! definitions that are output when you build your WASM.
//!
//! # Features
//!
//! * `default-exchanges` **(default)** - Include default exchanges and the related builder method
//! * `observable` **(default)** - Include support for observable and all related types. Includes
//! `tokio` on x86.

//#![warn(missing_docs)]
//#![deny(warnings)]

#[macro_use]
extern crate serde;
#[macro_use]
extern crate async_trait;

use std::{collections::HashMap, fmt, fmt::Display};
use types::*;

pub mod client;
pub mod default_exchanges;
mod error;
pub(crate) mod types;
pub mod utils;

pub use artemis_codegen_proc_macro::wasm_client;
pub use client::{Client, ClientBuilder};
pub use error::QueryError;
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "observable")]
pub use types::Observable;
pub use types::{
    DebugInfo, ExtensionMap, Extensions, HeaderPair, QueryOptions, RequestPolicy, ResultSource
};
#[cfg(target_arch = "wasm32")]
pub use utils::wasm;

/// Types used by custom exchanges. Regular users probably don't need these.
pub mod exchange {
    pub use crate::types::{
        Client, Exchange, ExchangeFactory, ExchangeResult, Extension, Operation, OperationMeta,
        OperationOptions, OperationResult, OperationType
    };
}

/// Types used only by the code generator. Exchanges may use these, but they shouldn't
/// be created/implemented manually.
pub mod codegen {
    pub use crate::types::{FieldSelector, QueryInfo};
}

/// The form in which queries are sent over HTTP in most implementations. This will be built using the [GraphQLQuery](./trait.GraphQLQuery.html) trait normally.
#[derive(Debug, Serialize, Clone)]
pub struct QueryBody<Variables: Serialize + Send + Sync + Clone> {
    /// The values for the variables. They must match those declared in the queries. This should be the `Variables` struct from the generated module corresponding to the query.
    pub variables: Variables,
    /// The GraphQL query, as a string.
    pub query: &'static str,
    /// The GraphQL operation name, as a string.
    #[serde(rename = "operationName")]
    pub operation_name: &'static str
}

/// A convenience trait that can be used to build a GraphQL request body.
/// This will be implemented for you by codegen. It is implemented on the struct you place the derive on.
pub trait GraphQLQuery: Send + Sync + 'static {
    /// The shape of the variables expected by the query. This should be a generated struct most of the time.
    type Variables: Serialize + Send + Sync + Clone + 'static;
    /// The top-level shape of the response data (the `data` field in the GraphQL response). In practice this should be generated, since it is hard to write by hand without error.
    type ResponseData: Serialize
        + DeserializeOwned
        + Send
        + Sync
        + Clone
        + 'static
        + QueryInfo<Self::Variables>;

    /// Produce a GraphQL query struct that can be JSON serialized and sent to a GraphQL API.
    fn build_query(variables: Self::Variables) -> (QueryBody<Self::Variables>, OperationMeta);

    fn selection(variables: &Self::Variables) -> Vec<FieldSelector> {
        <Self::ResponseData as QueryInfo<Self::Variables>>::selection(variables)
    }
}

/// The generic shape taken by the responses of GraphQL APIs.
///
/// This will generally be used with the `ResponseData` struct from a derived module.
///
/// [Spec](https://github.com/facebook/graphql/blob/master/spec/Section%207%20--%20Response.md)
///
/// ```
/// # use serde_json::json;
/// # use serde::Deserialize;
/// # use artemis::GraphQLQuery;
/// #
/// # #[derive(Debug, Deserialize, PartialEq, Clone)]
/// # struct User {
/// #     id: i32,
/// # }
/// #
/// # #[derive(Debug, Deserialize, PartialEq, Clone)]
/// # struct Dog {
/// #     name: String
/// # }
/// #
/// # #[derive(Debug, Deserialize, PartialEq, Clone)]
/// # struct ResponseData {
/// #     users: Vec<User>,
/// #     dogs: Vec<Dog>,
/// # }
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use artemis::Response;
///
/// let body: Response<ResponseData> = serde_json::from_value(json!({
///     "data": {
///         "users": [{"id": 13}],
///         "dogs": [{"name": "Strelka"}],
///     },
///     "errors": [],
/// }))?;
///
/// let expected: Response<ResponseData> = Response {
///     data: Some(ResponseData {
///         users: vec![User { id: 13 }],
///         dogs: vec![Dog { name: "Strelka".to_owned() }],
///     }),
///     errors: Some(vec![]),
///     debug_info: None
/// };
///
/// assert_eq!(body, expected);
///
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Response<Data: Clone> {
    /// The debug info if in test config, an empty struct otherwise
    #[serde(skip_deserializing, rename = "debugInfo")]
    pub debug_info: Option<DebugInfo>,
    /// The absent, partial or complete response data.
    pub data: Option<Data>,
    /// The top-level errors returned by the server.
    pub errors: Option<Vec<Error>>
}

/// An element in the top-level `errors` array of a response body.
///
/// This tries to be as close to the spec as possible.
///
/// [Spec](https://github.com/facebook/graphql/blob/master/spec/Section%207%20--%20Response.md)
///
///
/// ```
/// # use serde_json::json;
/// # use serde::Deserialize;
/// # use artemis::GraphQLQuery;
/// #
/// # #[derive(Debug, Deserialize, PartialEq, Clone)]
/// # struct ResponseData {
/// #     something: i32
/// # }
/// #
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use artemis::*;
///
/// let body: Response<ResponseData> = serde_json::from_value(json!({
///     "data": null,
///     "errors": [
///         {
///             "message": "The server crashed. Sorry.",
///             "locations": [{ "line": 1, "column": 1 }]
///         },
///         {
///             "message": "Seismic activity detected",
///             "path": ["underground", 20]
///         },
///      ],
/// }))?;
///
/// let expected: Response<ResponseData> = Response {
///     data: None,
///     errors: Some(vec![
///         Error {
///             message: "The server crashed. Sorry.".to_owned(),
///             locations: Some(vec![
///                 Location {
///                     line: 1,
///                     column: 1,
///                 }
///             ]),
///             path: None,
///             extensions: None,
///         },
///         Error {
///             message: "Seismic activity detected".to_owned(),
///             locations: None,
///             path: Some(vec![
///                 PathFragment::Key("underground".into()),
///                 PathFragment::Index(20),
///             ]),
///             extensions: None,
///         },
///     ]),
///     debug_info: None
/// };
///
/// assert_eq!(body, expected);
///
/// #     Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Error {
    /// The human-readable error message. This is the only required field.
    pub message: String,
    /// Which locations in the query the error applies to.
    pub locations: Option<Vec<Location>>,
    /// Which path in the query the error applies to, e.g. `["users", 0, "email"]`.
    pub path: Option<Vec<PathFragment>>,
    /// Additional errors. Their exact format is defined by the server.
    pub extensions: Option<HashMap<String, serde_json::Value>>
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use `/` as a separator like JSON Pointer.
        let path = self
            .path
            .as_ref()
            .map(|fragments| {
                fragments
                    .iter()
                    .fold(String::new(), |mut acc, item| {
                        acc.push_str(&format!("{}/", item));
                        acc
                    })
                    .trim_end_matches('/')
                    .to_string()
            })
            .unwrap_or_else(|| "<query>".to_string());

        // Get the location of the error. We'll use just the first location for this.
        let loc = self
            .locations
            .as_ref()
            .and_then(|locations| locations.iter().next())
            .cloned()
            .unwrap_or_else(Location::default);

        write!(f, "{}:{}:{}: {}", path, loc.line, loc.column, self.message)
    }
}

/// Part of a path in a query. It can be an object key or an array index. See [Error](./struct.Error.html).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PathFragment {
    /// A key inside an object
    Key(String),
    /// An index inside an array
    Index(i32)
}

/// Represents a location inside a query string. Used in errors. See [Error](./struct.Error.html).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Location {
    /// The line number in the query string where the error originated (starting from 1).
    pub line: i32,
    /// The column number in the query string where the error originated (starting from 1).
    pub column: i32
}

impl Display for PathFragment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PathFragment::Key(ref key) => write!(f, "{}", key),
            PathFragment::Index(ref idx) => write!(f, "{}", idx)
        }
    }
}
