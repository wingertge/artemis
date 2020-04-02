use crate::{codegen_options::*, operations::OperationType, utils::hash, CodegenError};
use heck::*;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::HashSet;

/// This struct contains the parameters necessary to generate code for a given operation.
pub(crate) struct GeneratedModule<'a> {
    pub operation: &'a crate::operations::Operation<'a>,
    pub query_string: &'a str,
    pub query_document: &'a graphql_parser::query::Document,
    pub schema: &'a crate::schema::Schema<'a>,
    pub options: &'a crate::GraphQLClientCodegenOptions
}

impl<'a> GeneratedModule<'a> {
    /// Generate the items for the variables and the response that will go inside the module.
    fn build_impls(&self) -> Result<(TokenStream, HashSet<String>), CodegenError> {
        Ok(crate::codegen::response_for_query(
            &self.schema,
            &self.query_document,
            &self.operation,
            &self.options
        )?)
    }

    fn build_typescript_defs(&self) -> Result<String, CodegenError> {
        Ok(crate::codegen::typescript_for_query(
            &self.schema,
            &self.query_document,
            &self.operation,
            &self.options
        )?)
    }

    /// Generate the module and all the code inside.
    pub(crate) fn to_token_stream(&self) -> Result<TokenStream, CodegenError> {
        let module_name = Ident::new(&self.operation.name.to_snake_case(), Span::call_site());
        let module_visibility = &self.options.module_visibility();
        let operation_name_literal = &self.operation.name;
        let operation_name_ident = self
            .options
            .normalization()
            .operation(operation_name_literal);
        let operation_name_ident = Ident::new(&operation_name_ident, Span::call_site());

        // Force cargo to refresh the generated code when the query file changes.
        let query_include = self
            .options
            .query_file()
            .map(|path| {
                let path = path.to_str();
                quote!(
                    const __QUERY_WORKAROUND: &str = include_str!(#path);
                )
            })
            .unwrap_or_else(|| quote! {});

        let query_string = &self.query_string;
        let query_string_hash = hash(query_string);
        let (impls, types) = self.build_impls()?;
        let typescript_definitions = self.build_typescript_defs()?;
        let operation_type = match &self.operation.operation_type {
            OperationType::Query => quote!(Query),
            OperationType::Mutation => quote!(Mutation),
            OperationType::Subscription => quote!(Subscription)
        };
        let operation_type = quote!(::artemis::exchange::OperationType::#operation_type);

        let struct_declaration: Option<_> = match self.options.mode {
            CodegenMode::Cli => Some(quote! {
                #module_visibility struct #operation_name_ident;
            }),
            // The struct is already present in derive mode.
            CodegenMode::Derive => None
        };

        let types: Vec<_> = types.into_iter().collect();
        let involved_types = quote!(vec![#(#types,)*]);

        let typescript = format!(
            r#"export namespace {operation_name} {{
            {definitions}
        }}"#,
            operation_name = operation_name_literal,
            definitions = typescript_definitions
        );
        let format_config =
            dprint_plugin_typescript::configuration::ConfigurationBuilder::new().build();
        println!("{}", typescript);
        let typescript =
            match dprint_plugin_typescript::format_text("temp.d.ts", &typescript, &format_config)
                .unwrap()
            {
                Some(formatted) => formatted,
                None => panic!("Typescript was ignored even though no ignore comment was present")
            };
        let typescript = quote! {
            #[cfg(target_arch = "wasm32")]
            use wasm_bindgen::prelude::*;

            #[cfg(target_arch = "wasm32")]
            #[wasm_bindgen(typescript_custom_section)]
            const TS_APPEND_CONTENT: &'static str = #typescript;
        };

        Ok(quote!(
            #typescript

            #[allow(clippy::all)]
            #struct_declaration

            #[allow(clippy::all)]
            #module_visibility mod #module_name {
                #![allow(dead_code)]

                pub const OPERATION_NAME: &str = #operation_name_literal;
                pub const QUERY: &str = #query_string;

                #query_include

                #impls
            }

            #[allow(clippy::all)]
            impl ::artemis::GraphQLQuery for #operation_name_ident {
                type Variables = #module_name::Variables;
                type ResponseData = #module_name::ResponseData;

                fn build_query(variables: Self::Variables) -> (::artemis::QueryBody<Self::Variables>, ::artemis::exchange::OperationMeta) {
                    let meta = ::artemis::exchange::OperationMeta {
                        query_key: #query_string_hash,
                        operation_type: #operation_type,
                        involved_types: #involved_types
                    };

                    let body = ::artemis::QueryBody {
                        variables,
                        query: #module_name::QUERY,
                        operation_name: #module_name::OPERATION_NAME,
                    };

                    (body, meta)
                }
            }
        ))
    }
}
