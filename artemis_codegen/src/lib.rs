#![recursion_limit = "128"]
//#![deny(missing_docs)]
#![deny(rust_2018_idioms)]
//#![deny(warnings)]

//! Crate for internal use by other graphql-client crates, for code generation.
//!
//! It is not meant to be used directly by users of the library.

use lazy_static::*; // TODO: Need to use later
use proc_macro2::TokenStream;
use quote::*;

mod codegen;
mod codegen_options;
/// Deprecation-related code
pub mod deprecation;
mod query;
/// Contains the `Schema` type and its implementation.
pub mod schema;

mod constants;
mod enums;
mod field_type;
mod fragments;
mod generated_module;
mod inputs;
mod interfaces;
mod introspection_response;
/// Normalization-related code
pub mod normalization;
mod objects;
mod operations;
mod scalars;
mod selection;
mod shared;
mod unions;
mod utils;
mod variables;
///
pub mod wasm;

#[cfg(test)]
mod tests;

mod extensions;

pub use crate::codegen_options::{CodegenMode, GraphQLClientCodegenOptions};

use crate::{unions::UnionError, utils::hash};
use std::{collections::HashMap, error::Error, fmt, io::Read};
use proc_macro2::Span;

type CacheMap<T> = std::sync::Mutex<HashMap<std::path::PathBuf, T>>;

/*// TODO: Replace with lazy_static once done, just for code completion
static SCHEMA_CACHE: CacheMap<String> = CacheMap::default();
static QUERY_CACHE: CacheMap<(String, graphql_parser::query::Document)> =
    CacheMap::default();*/
lazy_static! {
    static ref SCHEMA_CACHE: CacheMap<String> = CacheMap::default();
    static ref QUERY_CACHE: CacheMap<(String, graphql_parser::query::Document)> =
        CacheMap::default();
}

/// An error that happened during code generation
#[derive(Debug)]
pub enum CodegenError {
    /// An IO Error
    IoError(String, std::io::Error),
    /// An error that occurred while parsing a query
    QueryParsingError(graphql_parser::query::ParseError),
    /// An error that occurred while parsing the schema
    SchemaParsingError(graphql_parser::schema::ParseError),
    /// An error that occurred during serialization
    SerializationError(serde_json::Error),
    /// An internal error while parsing union types
    UnionError(unions::UnionError),
    /// A syntax error in the query
    SyntaxError(String),
    /// A type error in the query
    TypeError(String),
    /// An internal error, should not be returned in normal usage
    InternalError(String),
    /// An unimplemented feature
    UnimplementedError(String),
    /// Invalid inputs were passed
    InputError(String)
}
impl Error for CodegenError {}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::IoError(msg, inner) => write!(f, "io error: {}\n{}", msg, inner),
            CodegenError::QueryParsingError(inner) => write!(f, "parsing error: {}", inner),
            CodegenError::SchemaParsingError(inner) => write!(f, "parsing error: {}", inner),
            CodegenError::SerializationError(inner) => write!(f, "serialization error: {}", inner),
            CodegenError::UnionError(inner) => write!(f, "{}", inner),
            CodegenError::SyntaxError(msg) => write!(f, "syntax error: {}", msg),
            CodegenError::TypeError(msg) => write!(f, "type error: {}", msg),
            CodegenError::InternalError(msg) => write!(f, "internal error: {}", msg),
            CodegenError::UnimplementedError(msg) => write!(f, "unimplemented: {}", msg),
            CodegenError::InputError(msg) => write!(f, "invalid input: {}", msg)
        }
    }
}

impl From<UnionError> for CodegenError {
    fn from(e: UnionError) -> Self {
        CodegenError::UnionError(e)
    }
}

/// a
pub fn generate_root_token_stream(
    modules: Vec<String>,
    enum_variants: Vec<(syn::Ident, u32)>,
    options: GraphQLClientCodegenOptions
) -> TokenStream {
    let modules: Vec<_> = modules
        .iter()
        .map(|module_name| syn::Ident::new(module_name, Span::call_site()))
        .collect();
    let module_tokens = {
        let modules = modules.iter();
        quote! {
            #(pub mod #modules;)*
        }
    };
    let query_idents: Vec<_> = enum_variants.iter().map(|(ident, _)| ident).collect();
    let enum_variants: Vec<_> = enum_variants
        .iter()
        .map(|(ident, key)| quote!(#ident = #key))
        .collect();
    let wasm_mod = if options.wasm_bindgen {
        let enum_ = quote! {
            #[wasm_bindgen]
            #[derive(Copy, Clone, PartialEq)]
            #[repr(u32)]
            pub enum Queries {
                #(#enum_variants),*
            }

            impl QueryCollection for Queries {
                fn query<M: Exchange>(self, client: Arc<ClientImpl<M>>, variables: JsValue, options: QueryOptions)
                 -> ::futures::future::BoxFuture<'static, Result<JsValue, JsValue>> {
                    let fut = Box::pin(async move {
                        match self {
                            #(Queries::#query_idents => {
                                let variables = serde_wasm_bindgen::from_value
                                    ::<<#query_idents as GraphQLQuery>::Variables>(variables)
                                    .unwrap();
                                let response = client.query_with_options(#query_idents, variables, options).await;
                                response
                                    .map(|response| serde_wasm_bindgen::to_value(&response).unwrap())
                                    .map_err(|e| serde_wasm_bindgen::to_value(&JsQueryError::from(e)).unwrap())
                            }),*
                        }
                    });

                    Box::pin(::artemis::wasm::UnsafeSendFuture::new(fut))
                }

                fn subscribe<M: Exchange>(self, client: Arc<ClientImpl<M>>, variables: JsValue, callback: js_sys::Function, options: QueryOptions) {
                    match self {
                        #(Queries::#query_idents => {
                            let variables = serde_wasm_bindgen
                                ::from_value::<<#query_idents as GraphQLQuery>::Variables>(variables)
                                .unwrap();
                            let observable = client.subscribe_with_options(#query_idents, variables, options);
                            ::artemis::wasm::bind_stream(observable, callback);
                        }),*
                    }
                }
            }
        };
        let tokens = quote! {
            #[cfg(target_arch = "wasm32")]
            pub mod wasm {
                use wasm_bindgen::prelude::*;
                use std::sync::Arc;
                use artemis::{client::ClientImpl, GraphQLQuery, QueryOptions, wasm::{JsQueryError, QueryCollection}, exchange::Exchange};
                #(use super::#modules::*;)*

                #enum_
            }
        };

        //println!("{}", tokens);

        tokens
    } else {
        quote!()
    };

    quote! {
        #module_tokens
        #wasm_mod
    }
}

/// Generates Rust code given a query document, a schema and options.
pub fn generate_module_token_stream(
    query_path: std::path::PathBuf,
    schema_path: &std::path::Path,
    options: GraphQLClientCodegenOptions
) -> Result<(TokenStream, Vec<(syn::Ident, u32)>), CodegenError> {
    use std::collections::hash_map;
    // We need to qualify the query with the path to the crate it is part of
    let (query_string, query) = {
        let mut lock = QUERY_CACHE.lock().expect("query cache is poisoned");
        match lock.entry(query_path) {
            hash_map::Entry::Occupied(o) => o.get().clone(),
            hash_map::Entry::Vacant(v) => {
                let query_string = read_file(v.key())?;
                let query = graphql_parser::parse_query(&query_string)
                    .map_err(CodegenError::QueryParsingError)?;
                v.insert((query_string, query)).clone()
            }
        }
    };

    // Determine which operation we are generating code for. This will be used in operationName.
    let operations = options
        .operation_name
        .as_ref()
        .and_then(|operation_name| {
            codegen::select_operation(&query, &operation_name, options.normalization())
        })
        .map(|op| vec![op]);

    let operations = match (operations, &options.mode) {
        (Some(ops), _) => ops,
        (None, &CodegenMode::Cli) => codegen::all_operations(&query),
        (None, &CodegenMode::Derive) => {
            return Err(derive_operation_not_found_error(
                options.struct_ident(),
                &query
            ));
        }
    };

    let schema_extension = schema_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("INVALID");

    // Check the schema cache.
    let schema_string: String = {
        let mut lock = SCHEMA_CACHE.lock().expect("schema cache is poisoned");
        match lock.entry(schema_path.to_path_buf()) {
            hash_map::Entry::Occupied(o) => o.get().clone(),
            hash_map::Entry::Vacant(v) => {
                let schema_string = read_file(v.key())?;
                (*v.insert(schema_string)).to_string()
            }
        }
    };

    let parsed_schema = match schema_extension {
                        "graphql" | "gql" => {
                            let s = graphql_parser::schema::parse_schema(&schema_string).map_err(CodegenError::SchemaParsingError)?;
                            schema::ParsedSchema::GraphQLParser(s)
                        }
                        "json" => {
                            let parsed: crate::introspection_response::IntrospectionResponse = serde_json::from_str(&schema_string).map_err(CodegenError::SerializationError)?;
                            schema::ParsedSchema::Json(parsed)
                        }
                        extension => panic!("Unsupported extension for the GraphQL schema: {} (only .json and .graphql are supported)", extension)
                    };

    let schema = schema::Schema::from(&parsed_schema);

    // The generated modules.
    let mut modules = Vec::with_capacity(operations.len());
    let mut variants = if options.wasm_bindgen {
        Vec::with_capacity(operations.len())
    } else {
        Vec::new()
    };

    for operation in &operations {
        if options.wasm_bindgen {
            let operation_name_ident = options.normalization().operation(&operation.name);
            let key = hash(query_string.as_str());
            let variant_ident = syn::Ident::new(&operation_name_ident, Span::call_site());
            variants.push((variant_ident, key));
        }

        let generated = generated_module::GeneratedModule {
            query_string: query_string.as_str(),
            schema: &schema,
            query_document: &query,
            operation,
            options: &options
        }
        .to_token_stream()?;
        modules.push(generated);
    }

    let modules = quote! { #(#modules)* };

    Ok((modules, variants))
}

fn read_file(path: &std::path::Path) -> Result<String, CodegenError> {
    use std::fs;

    let mut out = String::new();
    let mut file = fs::File::open(path).map_err(|io_err| {
        let msg = format!(
            r#"
            Could not find file with path: {}
            Hint: file paths in the GraphQLQuery attribute are relative to the project root (location of the Cargo.toml). Example: query_path = "src/my_query.graphql".
            "#,
            path.display()
        );
        CodegenError::IoError(msg, io_err)
    })?;
    file.read_to_string(&mut out)
        .map_err(|e| CodegenError::IoError("".to_string(), e))?;
    Ok(out)
}

/// In derive mode, build an error when the operation with the same name as the struct is not found.
fn derive_operation_not_found_error(
    ident: Option<&proc_macro2::Ident>,
    query: &graphql_parser::query::Document
) -> CodegenError {
    use graphql_parser::query::*;

    let operation_name = ident.map(ToString::to_string);
    let struct_ident = operation_name.as_deref().unwrap_or("");

    let available_operations = query
        .definitions
        .iter()
        .filter_map(|definition| match definition {
            Definition::Operation(op) => match op {
                OperationDefinition::Mutation(m) => Some(m.name.as_ref().unwrap()),
                OperationDefinition::Query(m) => Some(m.name.as_ref().unwrap()),
                OperationDefinition::Subscription(m) => Some(m.name.as_ref().unwrap()),
                OperationDefinition::SelectionSet(_) => {
                    unreachable!("Bare selection sets are not supported.")
                }
            },
            _ => None
        })
        .fold(String::new(), |mut acc, item| {
            acc.push_str(&item);
            acc.push_str(", ");
            acc
        });

    let available_operations = available_operations.trim_end_matches(", ");

    CodegenError::TypeError(format!(
        "The struct name does not match any defined operation in the query file.\nStruct name: {}\nDefined operations: {}",
        struct_ident,
        available_operations,
    ))
}
