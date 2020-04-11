# artemis

A modern GraphQL Client with common built-in features
as well as the ability to extend its functionality through exchanges

## Getting Started

This crate needs two dependencies:  
The main crate in your regular dependencies, and [`artemis-build`](https://crates.io/crates/artemis-build) in your
`dev-dependencies`.

The first step is to write some queries in `.graphql` files and then add the following to your
`build.rs` (create it if necessary):

```rust
use artemis_build::CodegenBuilder;

fn main() {
    CodegenBuilder::new()
        .introspect_schema("http://localhost:8080/graphql", None, Vec::new())
        .unwrap()
        .add_query("queries/x.graphql")
        .with_out_dir("src/queries")
        .build()
        .unwrap();
}
```

Afterwards, you can use the crate in your application as such:

```rust
use artemis::Client;
use artemis_test::get_conference::{GetConference, get_conference::Variables};

let client = Client::builder("http://localhost:8080/graphql")
    .with_default_exchanges()
    .build();

let result = client.query(GetConference, Variables { id: "1".to_string() }).await.unwrap();
assert!(result.data.is_some());
```

For more info see the relevant method and struct documentation.

## Build

This crate uses code generation to take your GraphQL files and turn them into
strongly typed Rust modules. These contain the query struct, a zero-size type
such as `GetConference`, as well as a submodule containing the `Variables`,
any input types, the `ResponseData` type and any involved output types.

Having a strongly typed compile time representation with additional info
(such as the `__typename` of all involved types and an abstract selection tree)
means that the work the CPU has to do at runtime is very minimal,
only amounting to serialization, deserialization and simple lookups using
the statically generated data.

For details on how to use the query builder, see [artemis-build](../artemis_build/index.html)

## Exchanges

Exchanges are like a bi-directional middleware.
They act on both the incoming and outgoing queries,
passing them on if they can't return a result themselves.

There are three default exchanges, called in this order:

### DedupExchange

The deduplication exchange (`DedupExchange`) filters out unnecessary queries
by combining multiple identical queries into one. It does so by keeping track
of in-flight queries and, instead of firing off another identical query,
waiting for their results instead. This reduces network traffic,
especially in larger applications where the same query may be used in multiple
places and run multiple times simultaneously as a result.

### CacheExchange

The cache exchange is a very basic, un-normalized cache which eagerly invalidates queries.
It's focused on simplicity and correctness of data, so if a query uses any of the same types
as a mutation it will always be invalidated by it. This means that especially if you
have large amounts of different entities of the same type, this can become expensive quickly.
For a more advanced normalized cache that invalidates only directly related entities
see the `artemis-normalized-cache` crate.

### FetchExchange

The fetch exchange will serialize the query, send it over the network and deserialize the response.
This works on x86 using `reqwest`, or `fetch` if you're using WASM.
This should be your last exchange in the chain, as it never forwards a query.

## WASM

WASM support requires some minor boilerplate in your code.
First, there's a `wasm` module in your queries. this contains an automatically generated enum
containing all your queries. This is used for transmitting type data across the WASM
boundary.

Second, you have to use the [wasm_client! macro](../artemis_codegen_proc_macro/macro.wasm_client!.html)
to generate a WASM interop client that has hard-coded types for your queries, again, to
eliminate the unsupported generics and transmit type data across the boundary.
The queries type passed to the macro must be the enum generated as mentioned above.

Documentation of the JavaScript types and methods can be found in the TypeScript
definitions that are output when you build your WASM.

## Features

* `default-exchanges` **(default)** - Include default exchanges and the related builder method
* `observable` **(default)** - Include support for observable and all related types. Includes
`tokio` on x86.
