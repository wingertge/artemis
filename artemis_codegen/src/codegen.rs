use crate::{
    fragments::GqlFragment, normalization::Normalization, operations::Operation,
    query::QueryContext, schema, selection::Selection, CodegenError
};
use graphql_parser::query;
use proc_macro2::TokenStream;
use quote::*;
use std::collections::HashSet;

/// Selects the first operation matching `struct_name`. Returns `None` when the query document defines no operation, or when the selected operation does not match any defined operation.
pub(crate) fn select_operation<'query>(
    query: &'query query::Document,
    struct_name: &str,
    norm: Normalization
) -> Option<Operation<'query>> {
    let operations = all_operations(query);

    operations
        .iter()
        .find(|op| norm.operation(&op.name) == struct_name)
        .map(ToOwned::to_owned)
}

pub(crate) fn all_operations(query: &query::Document) -> Vec<Operation<'_>> {
    let mut operations: Vec<Operation<'_>> = Vec::new();

    for definition in &query.definitions {
        if let query::Definition::Operation(op) = definition {
            operations.push(op.into());
        }
    }
    operations
}

/// The main code generation function.
pub(crate) fn response_for_query(
    schema: &schema::Schema<'_>,
    query: &query::Document,
    operation: &Operation<'_>,
    options: &crate::GraphQLClientCodegenOptions
) -> Result<(TokenStream, HashSet<String>), CodegenError> {
    let mut context = QueryContext::new(
        schema,
        options.deprecation_strategy(),
        options.normalization(),
        options.include_query_info
    );

    if let Some(derives) = options.variables_derives() {
        context.ingest_variables_derives(&derives)?;
    }

    if let Some(derives) = options.response_derives() {
        context.ingest_response_derives(&derives)?;
    }

    let mut definitions: Vec<TokenStream> = Vec::new();
    let mut types: HashSet<String> = HashSet::new();

    for definition in &query.definitions {
        match definition {
            query::Definition::Operation(_op) => (),
            query::Definition::Fragment(fragment) => {
                let &query::TypeCondition::On(ref on) = &fragment.type_condition;
                let on = schema.fragment_target(on).ok_or_else(|| {
                    let msg = format!(
                        "Fragment {} is defined on unknown type: {}",
                        &fragment.name, on,
                    );
                    CodegenError::SyntaxError(msg)
                })?;
                context.fragments.insert(
                    &fragment.name,
                    GqlFragment {
                        name: &fragment.name,
                        selection: Selection::from(&fragment.selection_set),
                        on,
                        is_required: false.into()
                    }
                );
            }
        }
    }

    let response_type_name;
    let (response_data_selection, response_data_fields) = {
        let root_name = operation.root_name(&context.schema);
        let opt_definition = context.schema.objects.get(&root_name);
        let definition = if let Some(definition) = opt_definition {
            response_type_name = definition.name;
            definition
        } else {
            panic!(
                "operation type '{:?}' not in schema",
                operation.operation_type
            );
        };
        let prefix = &operation.name;
        let selection = &operation.selection;

        if operation.is_subscription() && selection.len() > 1 {
            return Err(CodegenError::SyntaxError(format!(
                "{}",
                crate::constants::MULTIPLE_SUBSCRIPTION_FIELDS_ERROR
            )));
        }

        let (tokens, used_types) =
            definition.field_impls_for_selection(&context, &selection, &prefix)?;
        definitions.extend(tokens);
        types.extend(used_types);
        definition.response_fields_for_selection(&context, &selection, &prefix)?
    };

    let enum_definitions = context.schema.enums.values().filter_map(|enm| {
        if enm.is_required.get() {
            Some(enm.to_rust(&context))
        } else {
            None
        }
    });
    let fragment_definitions: Result<Vec<TokenStream>, _> = context
        .fragments
        .values()
        .filter_map(|fragment| {
            if fragment.is_required.get() {
                Some(fragment.to_rust(&context))
            } else {
                None
            }
        })
        .collect();
    let fragment_definitions = fragment_definitions?;
    let variables_struct = operation.expand_variables(&context);

    let input_object_definitions: Result<Vec<TokenStream>, _> = context
        .schema
        .inputs
        .values()
        .filter_map(|i| {
            if i.is_required.get() {
                Some(i.to_rust(&context))
            } else {
                None
            }
        })
        .collect();
    let input_object_definitions = input_object_definitions?;

    let scalar_definitions: Vec<TokenStream> = context
        .schema
        .scalars
        .values()
        .filter_map(|s| {
            if s.is_required.get() {
                Some(s.to_rust(context.normalization))
            } else {
                None
            }
        })
        .collect();

    let response_derives = context.response_derives();

    let query_info = if context.include_query_info {
        quote! {
            impl ::artemis::QueryInfo<Variables> for ResponseData {
                fn typename(&self) -> &'static str {
                    #response_type_name
                }

                fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
                    vec![
                        #(#response_data_selection,)*
                    ]
                }
            }
        }
    } else {
        quote!()
    };

    let tokens = quote! {
        use serde::{Serialize, Deserialize};

        #[allow(dead_code)]
        type Boolean = bool;
        #[allow(dead_code)]
        type Float = f64;
        #[allow(dead_code)]
        type Int = i64;
        #[allow(dead_code)]
        type ID = String;

        #(#scalar_definitions)*

        #(#input_object_definitions)*

        #(#enum_definitions)*

        #(#fragment_definitions)*

        #(#definitions)*

        #variables_struct

        #response_derives

        pub struct ResponseData {
            #(#response_data_fields,)*
        }

        #query_info
    };

    Ok((tokens, types))
}
