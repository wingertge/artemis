use crate::{
    constants::*,
    deprecation::DeprecationStatus,
    field_type::FieldType,
    query::QueryContext,
    schema::Schema,
    selection::*,
    shared::{
        field_impls_for_selection, response_fields_for_selection,
        typescript_definitions_for_selection, typescript_fields_for_selection
    },
    CodegenError
};
use graphql_parser::schema;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::{cell::Cell, collections::HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct GqlObject<'schema> {
    pub description: Option<&'schema str>,
    pub fields: Vec<GqlObjectField<'schema>>,
    pub name: &'schema str,
    pub is_required: Cell<bool>
}

#[derive(Clone, Debug, PartialEq)]
pub struct GqlObjectField<'schema> {
    pub description: Option<&'schema str>,
    pub name: &'schema str,
    pub type_: FieldType<'schema>,
    pub deprecation: DeprecationStatus
}

fn parse_deprecation_info(field: &schema::Field) -> DeprecationStatus {
    let deprecated = field
        .directives
        .iter()
        .find(|x| x.name.to_lowercase() == "deprecated");
    let reason = if let Some(d) = deprecated {
        if let Some((_, value)) = d.arguments.iter().find(|x| x.0.to_lowercase() == "reason") {
            match value {
                schema::Value::String(reason) => Some(reason.clone()),
                schema::Value::Null => None,
                _ => panic!("deprecation reason is not a string")
            }
        } else {
            None
        }
    } else {
        None
    };
    match deprecated {
        Some(_) => DeprecationStatus::Deprecated(reason),
        None => DeprecationStatus::Current
    }
}

impl<'schema> GqlObject<'schema> {
    pub fn new(name: &'schema str, description: Option<&'schema str>) -> GqlObject<'schema> {
        GqlObject {
            description,
            name,
            fields: vec![typename_field()],
            is_required: false.into()
        }
    }

    pub fn from_graphql_parser_object(obj: &'schema schema::ObjectType) -> Self {
        let description = obj.description.as_deref();
        let mut item = GqlObject::new(&obj.name, description);
        item.fields.extend(obj.fields.iter().map(|f| {
            let deprecation = parse_deprecation_info(&f);
            GqlObjectField {
                description: f.description.as_deref(),
                name: &f.name,
                type_: FieldType::from(&f.field_type),
                deprecation
            }
        }));
        item
    }

    pub fn from_introspected_schema_json(
        obj: &'schema crate::introspection_response::FullType
    ) -> Self {
        let description = obj.description.as_deref();
        let mut item = GqlObject::new(obj.name.as_ref().expect("missing object name"), description);
        let fields = obj.fields.as_ref().unwrap().iter().filter_map(|t| {
            t.as_ref().map(|t| {
                let deprecation = if t.is_deprecated.unwrap_or(false) {
                    DeprecationStatus::Deprecated(t.deprecation_reason.clone())
                } else {
                    DeprecationStatus::Current
                };
                GqlObjectField {
                    description: t.description.as_deref(),
                    name: t.name.as_ref().expect("field name"),
                    type_: FieldType::from(t.type_.as_ref().expect("field type")),
                    deprecation
                }
            })
        });

        item.fields.extend(fields);

        item
    }

    pub(crate) fn require(&self, schema: &Schema<'_>) {
        if self.is_required.get() {
            return;
        }
        self.is_required.set(true);
        self.fields.iter().for_each(|field| {
            schema.require(&field.type_.inner_name_str());
        })
    }

    pub(crate) fn typescript_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str,
        include_typename: bool
    ) -> Result<String, CodegenError> {
        let fields = self.typescript_fields_for_selection(query_context, selection, prefix)?;
        let field_impls = self.typescript_definitions_for_selection(query_context, selection)?;
        let description = self
            .description
            .as_ref()
            .map(|desc| format!("/** {} */", desc))
            .unwrap_or_else(|| format!(""));
        let typename = if include_typename {
            format!("__typename: \"{}\",\n", prefix)
        } else {
            format!("")
        };

        let field_impls = if !field_impls.is_empty() {
            format!(
                r#"
                export namespace {name} {{
                    {field_impls}
                }}
                "#,
                name = prefix,
                field_impls = field_impls.join(",\n")
            )
        } else {
            format!("")
        };

        let tokens = format!(
            r#"
        {field_impls}

        {description}
        export interface {name} {{
            {typename}{fields}
        }}
        "#,
            field_impls = field_impls,
            description = description,
            name = prefix,
            typename = typename,
            fields = fields.join(",\n")
        );

        Ok(tokens)
    }

    pub(crate) fn response_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<(TokenStream, HashSet<String>), CodegenError> {
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
        let name = Ident::new(prefix, Span::call_site());
        let (field_infos, fields) =
            self.response_fields_for_selection(query_context, selection, prefix)?;
        let (field_impls, types) =
            self.field_impls_for_selection(query_context, selection, &prefix)?;
        let description = self.description.as_ref().map(|desc| quote!(#[doc = #desc]));

        let tokens = quote! {
            #(#field_impls)*

            #derives
            #wasm_derives
            #description
            pub struct #name {
                #(#fields,)*
            }

            impl #name {
                #[allow(unused_variables)]
                fn selection(variables: &Variables) -> Vec<::artemis::codegen::FieldSelector> {
                    vec![
                        #(#field_infos),*
                    ]
                }
            }
        };
        Ok((tokens, types))
    }

    pub(crate) fn field_impls_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<(Vec<TokenStream>, HashSet<String>), CodegenError> {
        field_impls_for_selection(&self.fields, query_context, selection, prefix)
    }

    pub(crate) fn typescript_definitions_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>
    ) -> Result<Vec<String>, CodegenError> {
        typescript_definitions_for_selection(&self.fields, query_context, selection)
    }

    pub(crate) fn typescript_fields_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<Vec<String>, CodegenError> {
        typescript_fields_for_selection(&self.name, &self.fields, query_context, selection, prefix)
    }

    pub(crate) fn response_fields_for_selection(
        &self,
        query_context: &QueryContext<'_, '_>,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<(Vec<TokenStream>, Vec<TokenStream>), CodegenError> {
        response_fields_for_selection(&self.name, &self.fields, query_context, selection, prefix)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use graphql_parser::{query, Pos};

    fn mock_field(directives: Vec<schema::Directive>) -> schema::Field {
        schema::Field {
            position: Pos::default(),
            description: None,
            name: "foo".to_string(),
            arguments: vec![],
            field_type: schema::Type::NamedType("x".to_string()),
            directives
        }
    }

    #[test]
    fn deprecation_no_reason() {
        let directive = schema::Directive {
            position: Pos::default(),
            name: "deprecated".to_string(),
            arguments: vec![]
        };
        let result = parse_deprecation_info(&mock_field(vec![directive]));
        assert_eq!(DeprecationStatus::Deprecated(None), result);
    }

    #[test]
    fn deprecation_with_reason() {
        let directive = schema::Directive {
            position: Pos::default(),
            name: "deprecated".to_string(),
            arguments: vec![(
                "reason".to_string(),
                query::Value::String("whatever".to_string())
            )]
        };
        let result = parse_deprecation_info(&mock_field(vec![directive]));
        assert_eq!(
            DeprecationStatus::Deprecated(Some("whatever".to_string())),
            result
        );
    }

    #[test]
    fn null_deprecation_reason() {
        let directive = schema::Directive {
            position: Pos::default(),
            name: "deprecated".to_string(),
            arguments: vec![("reason".to_string(), query::Value::Null)]
        };
        let result = parse_deprecation_info(&mock_field(vec![directive]));
        assert_eq!(DeprecationStatus::Deprecated(None), result);
    }

    #[test]
    #[should_panic]
    fn invalid_deprecation_reason() {
        let directive = schema::Directive {
            position: Pos::default(),
            name: "deprecated".to_string(),
            arguments: vec![("reason".to_string(), query::Value::Boolean(true))]
        };
        let _ = parse_deprecation_info(&mock_field(vec![directive]));
    }

    #[test]
    fn no_deprecation() {
        let result = parse_deprecation_info(&mock_field(vec![]));
        assert_eq!(DeprecationStatus::Current, result);
    }
}
