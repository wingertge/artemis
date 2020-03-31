use crate::{
    constants::TYPENAME_FIELD,
    objects::GqlObjectField,
    query::QueryContext,
    selection::{Selection, SelectionField, SelectionFragmentSpread, SelectionItem},
    shared::*,
    unions::{union_variants, union_variants_typescript},
    CodegenError
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::{cell::Cell, collections::HashSet};

/// A GraphQL interface (simplified schema representation).
///
/// In the generated code, fragments nesting is preserved, including for selection on union variants. See the tests in the graphql client crate for examples.
#[derive(Debug, Clone, PartialEq)]
pub struct GqlInterface<'schema> {
    /// The documentation for the interface. Extracted from the schema.
    pub description: Option<&'schema str>,
    /// The set of object types implementing this interface.
    pub implemented_by: HashSet<&'schema str>,
    /// The name of the interface. Should match 1-to-1 to its name in the GraphQL schema.
    pub name: &'schema str,
    /// The interface's fields. Analogous to object fields.
    pub fields: Vec<GqlObjectField<'schema>>,
    pub is_required: Cell<bool>
}

impl<'schema> GqlInterface<'schema> {
    /// filters the selection to keep only the fields that refer to the interface's own.
    ///
    /// This does not include the __typename field because it is translated into the `on` enum.
    fn object_selection<'query>(
        &self,
        selection: &'query Selection<'query>,
        query_context: &QueryContext<'_, '_>
    ) -> Selection<'query> {
        (&selection)
            .into_iter()
            // Only keep what we can handle
            .filter(|f| match f {
                SelectionItem::Field(f) => f.name != TYPENAME_FIELD,
                SelectionItem::FragmentSpread(SelectionFragmentSpread { fragment_name }) => {
                    // only if the fragment refers to the interface’s own fields (to take into account type-refining fragments)
                    let fragment = query_context
                        .fragments
                        .get(fragment_name)
                        .ok_or_else(|| {
                            CodegenError::TypeError(format!("Unknown fragment: {}", &fragment_name))
                        })
                        // TODO: fix this
                        .unwrap();

                    fragment.on.name() == self.name
                }
                SelectionItem::InlineFragment(_) => false
            })
            .map(|a| (*a).clone())
            .collect()
    }

    fn union_selection<'query>(
        &self,
        selection: &'query Selection<'_>,
        query_context: &QueryContext<'_, '_>
    ) -> Selection<'query> {
        (&selection)
            .into_iter()
            // Only keep what we can handle
            .filter(|f| match f {
                SelectionItem::InlineFragment(_) => true,
                SelectionItem::FragmentSpread(SelectionFragmentSpread { fragment_name }) => {
                    let fragment = query_context
                        .fragments
                        .get(fragment_name)
                        .ok_or_else(|| {
                            CodegenError::TypeError(format!("Unknown fragment: {}", &fragment_name))
                        })
                        // TODO: fix this
                        .unwrap();

                    // only the fragments _not_ on the interface
                    fragment.on.name() != self.name
                }
                SelectionItem::Field(SelectionField { name, .. }) => *name == "__typename"
            })
            .map(|a| (*a).clone())
            .collect()
    }

    /// Create an empty interface. This needs to be mutated before it is useful.
    pub(crate) fn new(
        name: &'schema str,
        description: Option<&'schema str>
    ) -> GqlInterface<'schema> {
        GqlInterface {
            name,
            description,
            implemented_by: HashSet::new(),
            fields: vec![],
            is_required: false.into()
        }
    }

    pub(crate) fn typescript_definitions_for_selection(
        &self,
        context: &QueryContext<'_, '_>,
        selection: &Selection<'_>
    ) -> Result<Vec<String>, CodegenError> {
        crate::shared::typescript_definitions_for_selection(
            &self.fields,
            context,
            &self.object_selection(selection, context)
        )
    }

    /// The generated code for each of the selected field's types. See [shared::field_impls_for_selection].
    pub(crate) fn field_impls_for_selection(
        &self,
        context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<(Vec<TokenStream>, HashSet<String>), CodegenError> {
        crate::shared::field_impls_for_selection(
            &self.fields,
            context,
            &self.object_selection(selection, context),
            prefix
        )
    }

    pub(crate) fn typescript_fields_for_selection(
        &self,
        context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<Vec<String>, CodegenError> {
        typescript_fields_for_selection(
            &self.name,
            &self.fields,
            context,
            &self.object_selection(selection, context),
            prefix
        )
    }

    /// The code for the interface's corresponding struct's fields.
    pub(crate) fn response_fields_for_selection(
        &self,
        context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), CodegenError> {
        response_fields_for_selection(
            &self.name,
            &self.fields,
            context,
            &self.object_selection(selection, context),
            prefix
        )
    }

    pub(crate) fn typescript_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<String, CodegenError> {
        selection.extract_typename(query_context).ok_or_else(|| {
            CodegenError::InternalError(format!(
                "Missing __typename in selection for the {} interface (type: {})",
                prefix, self.name
            ))
        })?;

        let object_fields =
            self.typescript_fields_for_selection(query_context, &selection, prefix)?;

        let object_children =
            self.typescript_definitions_for_selection(query_context, &selection)?;

        let union_selection = self.union_selection(&selection, &query_context);

        let (mut union_variants, union_children, used_variants) =
            union_variants_typescript(&union_selection, query_context, prefix, &self.name)?;

        for used_variant in used_variants.iter() {
            if !self.implemented_by.contains(used_variant) {
                return Err(CodegenError::TypeError(format!(
                    "Type {} does not implement the {} interface",
                    used_variant, self.name,
                )));
            }
        }

        // Add the non-selected variants to the generated enum's variants.
        union_variants.extend(
            self.implemented_by
                .iter()
                .filter(|obj| used_variants.iter().find(|v| v == obj).is_none())
                .map(|v| (*v).to_string())
        );

        let attached_enum_name = format!("{}On", prefix);
        let attached_enum = if selection.extract_typename(query_context).is_some() {
            format!(
                "export type {name} = {variants}",
                name = attached_enum_name,
                variants = union_variants.join(" | ")
            )
        } else {
            format!("")
        };

        let type_name = self.name;

        let children = if object_children.len() + union_children.len() > 0 {
            format!(
                r#"
                export namespace {name} {{
                    {objects}
                    {unions}
                }}
                "#,
                name = prefix,
                objects = object_children.join("\n"),
                unions = union_children.join("\n")
            )
        } else {
            format!("")
        };

        let tokens = format!(
            r#"
            {children}

            {_enum}

            export interface {name} {{
                {fields}
            }}
        "#,
            children = children,
            _enum = attached_enum,
            name = type_name,
            fields = object_fields.join(",\n")
        );

        Ok(tokens)
    }

    /// Generate all the code for the interface.
    pub(crate) fn response_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<(TokenStream, HashSet<String>), CodegenError> {
        let name = Ident::new(&prefix, Span::call_site());
        let derives = query_context.response_derives();
        let wasm_derives = if query_context.wasm_bindgen {
            let filtered: Vec<_> = vec!["Serialize"]
                .into_iter()
                .map(|def| syn::Ident::new(def, Span::call_site()))
                .filter(|def| !query_context.response_derives.contains(def))
                .collect();
            if !filtered.is_empty() {
                quote!(#[cfg_attr(target_arch = "wasm32", derive(#(#filtered),*))])
            } else {
                quote!()
            }
        } else {
            quote!()
        };

        selection.extract_typename(query_context).ok_or_else(|| {
            CodegenError::InternalError(format!(
                "Missing __typename in selection for the {} interface (type: {})",
                prefix, self.name
            ))
        })?;

        let (selection_fields, object_fields) =
            self.response_fields_for_selection(query_context, &selection, prefix)?;

        let (object_children, _) =
            self.field_impls_for_selection(query_context, &selection, prefix)?;

        let union_selection = self.union_selection(&selection, &query_context);

        let (mut union_variants, union_children, used_variants, types) =
            union_variants(&union_selection, query_context, prefix, &self.name)?;

        for used_variant in used_variants.iter() {
            if !self.implemented_by.contains(used_variant) {
                return Err(CodegenError::TypeError(format!(
                    "Type {} does not implement the {} interface",
                    used_variant, self.name,
                )));
            }
        }

        // Add the non-selected variants to the generated enum's variants.
        union_variants.extend(
            self.implemented_by
                .iter()
                .filter(|obj| used_variants.iter().find(|v| v == obj).is_none())
                .map(|v| {
                    let v = Ident::new(v, Span::call_site());
                    quote!(#v)
                })
        );

        let attached_enum_name = Ident::new(&format!("{}On", name), Span::call_site());
        let (attached_enum, last_object_field) =
            if selection.extract_typename(query_context).is_some() {
                let attached_enum = quote! {
                    #derives
                    #wasm_derives
                    #[serde(tag = "__typename")]
                    pub enum #attached_enum_name {
                        #(#union_variants,)*
                    }
                };
                let last_object_field = quote!(#[serde(flatten)] pub on: #attached_enum_name,);
                (Some(attached_enum), Some(last_object_field))
            } else {
                (None, None)
            };

        let tokens = quote! {

            #(#object_children)*

            #(#union_children)*

            #attached_enum

            #derives
            #wasm_derives
            pub struct #name {
                #(#object_fields,)*
                #last_object_field
            }

            impl #name {
                #[allow(unused_variables)]
                fn selection(variables: &Variables) -> Vec<::artemis::FieldSelector> {
                    vec![
                        #(#selection_fields,)*
                    ]
                }
            }
        };

        Ok((tokens, types))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // to be improved
    #[test]
    fn union_selection_works() {
        let iface = GqlInterface {
            description: None,
            implemented_by: HashSet::new(),
            name: "MyInterface",
            fields: vec![],
            is_required: Cell::new(true)
        };

        let schema = crate::schema::Schema::new();
        let context = QueryContext::new_empty(&schema);

        let typename_field =
            crate::selection::SelectionItem::Field(crate::selection::SelectionField {
                alias: None,
                name: "__typename",
                fields: Selection::new_empty(),
                arguments: Vec::new()
            });
        let selection = Selection::from_vec(vec![typename_field.clone()]);

        assert_eq!(
            iface.union_selection(&selection, &context),
            Selection::from_vec(vec![typename_field])
        );
    }

    // to be improved
    #[test]
    fn object_selection_works() {
        let iface = GqlInterface {
            description: None,
            implemented_by: HashSet::new(),
            name: "MyInterface",
            fields: vec![],
            is_required: Cell::new(true)
        };

        let schema = crate::schema::Schema::new();
        let context = QueryContext::new_empty(&schema);

        let typename_field =
            crate::selection::SelectionItem::Field(crate::selection::SelectionField {
                alias: None,
                name: "__typename",
                fields: Selection::new_empty(),
                arguments: Vec::new()
            });
        let selection: Selection<'_> = vec![typename_field].into_iter().collect();

        assert_eq!(
            iface.object_selection(&selection, &context),
            Selection::new_empty()
        );
    }
}
