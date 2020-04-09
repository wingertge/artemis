use crate::{
    deprecation::{DeprecationStatus, DeprecationStrategy},
    fragments::GqlFragment,
    objects::GqlObjectField,
    query::QueryContext,
    selection::*,
    CodegenError
};
use graphql_parser::schema::Value;
use heck::{CamelCase, SnakeCase};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::{BTreeMap, HashSet};

// List of keywords based on https://doc.rust-lang.org/grammar.html#keywords
const RUST_KEYWORDS: &[&str] = &[
    "abstract",
    "alignof",
    "as",
    "async",
    "await",
    "become",
    "box",
    "break",
    "const",
    "continue",
    "crate",
    "do",
    "else",
    "enum",
    "extern crate",
    "extern",
    "false",
    "final",
    "fn",
    "for",
    "for",
    "if let",
    "if",
    "if",
    "impl",
    "impl",
    "in",
    "let",
    "loop",
    "macro",
    "match",
    "mod",
    "move",
    "mut",
    "offsetof",
    "override",
    "priv",
    "proc",
    "pub",
    "pure",
    "ref",
    "return",
    "self",
    "sizeof",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "typeof",
    "unsafe",
    "unsized",
    "use",
    "use",
    "virtual",
    "where",
    "while",
    "yield"
];

pub(crate) fn keyword_replace(needle: &str) -> String {
    match RUST_KEYWORDS.binary_search(&needle) {
        Ok(index) => [RUST_KEYWORDS[index], "_"].concat(),
        Err(_) => needle.to_owned()
    }
}

pub(crate) fn render_typescript_field(
    field_name: &str,
    field_type: &str,
    description: Option<&str>,
    status: &DeprecationStatus,
    strategy: &DeprecationStrategy
) -> Option<String> {
    #[allow(unused_assignments)]
    let mut deprecation = (false, "");
    match (status, strategy) {
        // If the field is deprecated and we are denying usage, don't generate the
        // field in rust at all and short-circuit.
        (DeprecationStatus::Deprecated(_), DeprecationStrategy::Deny) => return None,
        // A reason was provided, translate it to a note.
        (DeprecationStatus::Deprecated(Some(reason)), DeprecationStrategy::Warn) => {
            deprecation = (true, reason)
        }
        // No reason provided, just mark as deprecated.
        (DeprecationStatus::Deprecated(None), DeprecationStrategy::Warn) => {
            deprecation = (true, "")
        }
        _ => {}
    };

    let description = description.map(|s| format!("\n* {}", s));
    let doc = if description.is_some() || deprecation.0 {
        let deprecation = if deprecation.0 {
            let reason = deprecation.1;
            format!("\n* @deprecated {} ", reason)
        } else {
            String::new()
        };
        format!(
            r#"
            /**{description}{deprecation}
            */
        "#,
            description = description.unwrap_or_else(String::new),
            deprecation = deprecation
        )
    } else {
        String::new()
    };

    Some(format!(
        r#"
        {doc}
        {field_name}: {field_type}
    "#,
        doc = doc,
        field_name = field_name,
        field_type = field_type
    ))
}

pub(crate) fn render_object_field(
    field_name: &str,
    field_type: &TokenStream,
    description: Option<&str>,
    status: &DeprecationStatus,
    strategy: &DeprecationStrategy
) -> Option<TokenStream> {
    #[allow(unused_assignments)]
    let mut deprecation = quote!();
    match (status, strategy) {
        // If the field is deprecated and we are denying usage, don't generate the
        // field in rust at all and short-circuit.
        (DeprecationStatus::Deprecated(_), DeprecationStrategy::Deny) => return None,
        // Everything is allowed so there is nothing to do.
        (_, DeprecationStrategy::Allow) => deprecation = quote!(),
        // Current so there is nothing to do.
        (DeprecationStatus::Current, _) => deprecation = quote!(),
        // A reason was provided, translate it to a note.
        (DeprecationStatus::Deprecated(Some(reason)), DeprecationStrategy::Warn) => {
            deprecation = quote!(#[deprecated(note = #reason)])
        }
        // No reason provided, just mark as deprecated.
        (DeprecationStatus::Deprecated(None), DeprecationStrategy::Warn) => {
            deprecation = quote!(#[deprecated])
        }
    };

    let description = description.map(|s| quote!(#[doc = #s]));
    let rust_safe_field_name = keyword_replace(&field_name.to_snake_case());
    let name_ident = Ident::new(&rust_safe_field_name, Span::call_site());
    let rename = crate::shared::field_rename_annotation(&field_name, &rust_safe_field_name);

    Some(quote!(#description #deprecation #rename pub #name_ident: #field_type))
}

pub(crate) fn typescript_definitions_for_selection(
    fields: &[GqlObjectField<'_>],
    context: &QueryContext<'_, '_>,
    selection: &Selection<'_>
) -> Result<Vec<String>, CodegenError> {
    let tokens: Vec<String> = (&selection)
        .into_iter()
        .map(|selected| {
            if let SelectionItem::Field(selected) = selected {
                let name = &selected.name;
                let alias = selected.alias.as_ref().unwrap_or(name);

                let ty = fields
                    .iter()
                    .find(|f| &f.name == name)
                    .ok_or_else(|| {
                        CodegenError::TypeError(format!("could not find field `{}`", name))
                    })?
                    .type_
                    .inner_name_str();
                let prefix = alias.to_camel_case();
                context.maybe_expand_typescript_field(&ty, &selected.fields, &prefix)
            } else {
                Ok(None)
            }
        })
        .filter_map(|i| i.transpose())
        .collect::<Result<Vec<String>, CodegenError>>()?;

    Ok(tokens)
}

pub(crate) fn field_impls_for_selection(
    fields: &[GqlObjectField<'_>],
    context: &QueryContext<'_, '_>,
    selection: &Selection<'_>,
    prefix: &str
) -> Result<(Vec<TokenStream>, HashSet<String>), CodegenError> {
    let results: Vec<(TokenStream, HashSet<String>)> = (&selection)
        .into_iter()
        .map(|selected| {
            if let SelectionItem::Field(selected) = selected {
                let name = &selected.name;
                let alias = selected.alias.as_ref().unwrap_or(name);

                let ty = fields
                    .iter()
                    .find(|f| &f.name == name)
                    .ok_or_else(|| {
                        CodegenError::TypeError(format!("could not find field `{}`", name))
                    })?
                    .type_
                    .inner_name_str();
                let prefix = format!("{}{}", prefix.to_camel_case(), alias.to_camel_case());
                context.maybe_expand_field(&ty, &selected.fields, &prefix)
            } else {
                Ok(None)
            }
        })
        .filter_map(|i| i.transpose())
        .collect::<Result<Vec<(TokenStream, HashSet<String>)>, CodegenError>>()?;

    let types: HashSet<String> = results
        .iter()
        .map(|(_, types)| types.clone())
        .flatten()
        .collect();

    let tokens = results.into_iter().map(|(tokens, _)| tokens).collect();

    Ok((tokens, types))
}

#[derive(PartialEq, Debug, Clone)]
pub enum ArgumentValue {
    Variable(String),
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Enum(String),
    List(Vec<ArgumentValue>),
    Object(BTreeMap<String, ArgumentValue>)
}

pub trait ToRust {
    fn to_rust(&self) -> Option<TokenStream>;
}

impl ToRust for Vec<(String, ArgumentValue)> {
    fn to_rust(&self) -> Option<TokenStream> {
        if self.is_empty() {
            return None;
        }

        let mut placeholder_idents = Vec::new();
        let fields: Vec<String> = self
            .iter()
            .map(|(name, value)| {
                let (formatted, idents) = value.format(name);
                placeholder_idents.extend(idents);
                formatted
            })
            .collect();
        let fields = fields.join(",");
        let formatted_vars = format!("({})", fields);

        let idents = if !placeholder_idents.is_empty() {
            quote!(,#(&variables.#placeholder_idents),*)
        } else {
            quote!()
        };

        Some(quote! {
            format!(#formatted_vars #idents)
        })
    }
}

impl ArgumentValue {
    fn format(&self, name: &str) -> (String, Vec<Ident>) {
        let mut placeholder_idents = Vec::new();
        let formatted = match self {
            ArgumentValue::Int(i) => format!("{}:{}", name, i),
            ArgumentValue::Float(i) => format!("{}:{}", name, i),
            ArgumentValue::String(i) => format!("{}:{}", name, i),
            ArgumentValue::Boolean(i) => format!("{}:{}", name, i),
            ArgumentValue::Null => format!("{}:null", name),
            ArgumentValue::Enum(i) => format!("{}:{}", name, i),
            ArgumentValue::List(list) => {
                let entries: Vec<String> = list
                    .iter()
                    .map(|entry| {
                        let key = String::new();
                        let (formatted, idents) = entry.format(&key);
                        let formatted = formatted.replace(":", "");
                        placeholder_idents.extend(idents);
                        formatted
                    })
                    .collect();
                let entries = entries.join(",");
                format!("[{}]", entries)
            }
            ArgumentValue::Object(map) => {
                let entries: Vec<String> = map
                    .iter()
                    .map(|(key, value)| {
                        let (formatted, idents) = value.format(key);
                        placeholder_idents.extend(idents);
                        formatted
                    })
                    .collect();
                let entries = entries.join(",");
                format!("{{{{{}}}}}", entries)
            }
            ArgumentValue::Variable(var_name) => {
                placeholder_idents.push(Ident::new(var_name, Span::call_site()));
                format!("{}:{{:?}}", name)
            }
        };

        (formatted, placeholder_idents)
    }
}

impl From<Value> for ArgumentValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Variable(x) => ArgumentValue::Variable(x),
            Value::Int(x) => ArgumentValue::Int(x.as_i64().unwrap()), //This is always Some
            Value::Float(x) => ArgumentValue::Float(x),
            Value::String(x) => ArgumentValue::String(x),
            Value::Boolean(x) => ArgumentValue::Boolean(x),
            Value::Null => ArgumentValue::Null,
            Value::Enum(x) => ArgumentValue::Enum(x),
            Value::List(list) => ArgumentValue::List(list.into_iter().map(Into::into).collect()),
            Value::Object(object) => {
                let map = object
                    .into_iter()
                    .map(|(key, value)| (key, value.into()))
                    .collect();
                ArgumentValue::Object(map)
            }
        }
    }
}

pub(crate) fn typescript_fields_for_selection(
    type_name: &str,
    schema_fields: &[GqlObjectField<'_>],
    context: &QueryContext<'_, '_>,
    selection: &Selection<'_>,
    prefix: &str
) -> Result<Vec<String>, CodegenError> {
    let field_defs: Result<Vec<String>, CodegenError> = (&selection)
        .into_iter()
        .map(|item| match item {
            SelectionItem::Field(f) => {
                let name = &f.name;
                let alias = f.alias.as_ref().unwrap_or(name);

                let schema_field = schema_fields
                    .iter()
                    .find(|field| &field.name == name)
                    .ok_or_else(|| {
                        CodegenError::TypeError(format!(
                            "Could not find field `{}` on `{}`. Available fields: `{}`.",
                            *name,
                            type_name,
                            schema_fields
                                .iter()
                                .map(|ref field| &field.name)
                                .fold(String::new(), |mut acc, item| {
                                    acc.push_str(item);
                                    acc.push_str(", ");
                                    acc
                                })
                                .trim_end_matches(", ")
                        ))
                    })?;
                let ty_name = if prefix.is_empty() {
                    alias.to_camel_case()
                } else {
                    format!("{}.{}", prefix.to_camel_case(), alias.to_camel_case())
                };
                let ty = schema_field.type_.to_typescript(context, &ty_name);

                Ok(render_typescript_field(
                    alias,
                    &ty,
                    schema_field.description.as_ref().cloned(),
                    &schema_field.deprecation,
                    &context.deprecation_strategy
                ))
            }
            SelectionItem::FragmentSpread(fragment) => {
                context.require_fragment(&fragment.fragment_name);
                let fragment_from_context = context
                    .fragments
                    .get(&fragment.fragment_name)
                    .ok_or_else(|| {
                        CodegenError::TypeError(format!(
                            "Unknown fragment: {}",
                            &fragment.fragment_name
                        ))
                    })?;
                let fields = typescript_fields_for_selection(
                    fragment.fragment_name,
                    schema_fields,
                    context,
                    &fragment_from_context.selection,
                    prefix
                )?;
                Ok(Some(fields.join(",")))
            }
            SelectionItem::InlineFragment(_) => Err(CodegenError::UnimplementedError(
                "inline fragment on object field".to_string()
            ))
        })
        .filter_map(|x| x.transpose())
        .collect();

    Ok(field_defs?)
}

pub(crate) fn response_fields_for_selection(
    type_name: &str,
    schema_fields: &[GqlObjectField<'_>],
    context: &QueryContext<'_, '_>,
    selection: &Selection<'_>,
    prefix: &str
) -> Result<(Vec<TokenStream>, Vec<TokenStream>), CodegenError> {
    let mut selectors = Vec::new();

    let field_defs: Result<Vec<TokenStream>, CodegenError> = (&selection)
        .into_iter()
        .map(|item| match item {
            SelectionItem::Field(f) => {
                let name = &f.name;
                let alias = f.alias.as_ref().unwrap_or(name);

                let schema_field = schema_fields
                    .iter()
                    .find(|field| &field.name == name)
                    .ok_or_else(|| {
                        CodegenError::TypeError(format!(
                            "Could not find field `{}` on `{}`. Available fields: `{}`.",
                            *name,
                            type_name,
                            schema_fields
                                .iter()
                                .map(|ref field| &field.name)
                                .fold(String::new(), |mut acc, item| {
                                    acc.push_str(item);
                                    acc.push_str(", ");
                                    acc
                                })
                                .trim_end_matches(", ")
                        ))
                    })?;
                let prefix = format!("{}{}", prefix.to_camel_case(), alias.to_camel_case());
                let ty = schema_field.type_.to_rust(context, &prefix);
                let field_selector =
                    schema_field
                        .type_
                        .field_selector(context, &prefix, name, f.arguments.clone());

                selectors.push(field_selector);

                Ok(render_object_field(
                    alias,
                    &ty,
                    schema_field.description.as_ref().cloned(),
                    &schema_field.deprecation,
                    &context.deprecation_strategy
                ))
            }
            SelectionItem::FragmentSpread(fragment) => {
                let field_name =
                    Ident::new(&fragment.fragment_name.to_snake_case(), Span::call_site());
                context.require_fragment(&fragment.fragment_name);
                let fragment_from_context = context
                    .fragments
                    .get(&fragment.fragment_name)
                    .ok_or_else(|| {
                        CodegenError::TypeError(format!(
                            "Unknown fragment: {}",
                            &fragment.fragment_name
                        ))
                    })?;
                let fragment_selectors =
                    get_fragment_selectors(fragment_from_context, schema_fields, context);
                let type_name = Ident::new(&fragment.fragment_name, Span::call_site());
                let type_name = if fragment_from_context.is_recursive() {
                    quote!(Box<#type_name>)
                } else {
                    quote!(#type_name)
                };

                selectors.extend(fragment_selectors);

                Ok(Some(quote! {
                    #[serde(flatten)]
                    pub #field_name: #type_name
                }))
            }
            SelectionItem::InlineFragment(_) => Err(CodegenError::UnimplementedError(
                "inline fragment on object field".to_string()
            ))
        })
        .filter_map(|x| match x {
            // Remove empty fields so callers always know a field has some
            // tokens.
            Ok(f) => f.map(Ok),
            Err(err) => Some(Err(err))
        })
        .collect();

    Ok((selectors, field_defs?))
}

fn get_fragment_selectors(
    fragment: &GqlFragment<'_>,
    schema_fields: &[GqlObjectField<'_>],
    context: &QueryContext<'_, '_>
) -> Vec<TokenStream> {
    fragment
        .selection
        .0
        .iter()
        .flat_map(|selector| match selector {
            SelectionItem::Field(field) => {
                let name = field.name;
                let alias = field.alias.unwrap_or(name);

                let schema_field = schema_fields
                    .iter()
                    .find(|field| field.name == name)
                    .ok_or_else(|| CodegenError::TypeError("".to_string()))
                    .unwrap();
                let alias = alias.to_camel_case();
                let field_selector = schema_field.type_.field_selector(
                    context,
                    &alias,
                    name,
                    field.arguments.to_vec()
                );

                vec![field_selector]
            }
            SelectionItem::FragmentSpread(fragment) => {
                let fragment_from_context = context
                    .fragments
                    .get(&fragment.fragment_name)
                    .ok_or_else(|| {
                        CodegenError::TypeError(format!(
                            "Unknown fragment: {}",
                            &fragment.fragment_name
                        ))
                    })
                    .unwrap();
                get_fragment_selectors(fragment_from_context, schema_fields, context)
            }
            SelectionItem::InlineFragment(_) => panic!("Inline fragment on object field")
        })
        .collect()
}

/// Given the GraphQL schema name for an object/interface/input object field and
/// the equivalent rust name, produces a serde annotation to map them during
/// (de)serialization if it is necessary, otherwise an empty TokenStream.
pub(crate) fn field_rename_annotation(graphql_name: &str, rust_name: &str) -> Option<TokenStream> {
    if graphql_name != rust_name {
        Some(quote!(#[serde(rename = #graphql_name)]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        deprecation::DeprecationStatus,
        field_type::FieldType,
        fragments::{FragmentTarget, GqlFragment},
        objects::{GqlObject, GqlObjectField},
        query::QueryContext,
        selection::{Selection, SelectionField, SelectionFragmentSpread, SelectionItem},
        shared::get_fragment_selectors
    };
    use quote::quote;
    use std::cell::Cell;

    #[test]
    fn fragment_selector_returns_flattened_selectors() {
        let schema = crate::schema::Schema::new();
        let ctx = QueryContext::new_empty(&schema);
        let fragment_on = GqlObject {
            name: "Test",
            is_required: Cell::new(true),
            description: None,
            fields: Vec::new()
        };
        let schema_fields = vec![
            GqlObjectField {
                name: "field_1",
                description: None,
                deprecation: DeprecationStatus::Current,
                type_: FieldType::new("ID")
            },
            GqlObjectField {
                name: "field_2",
                description: None,
                deprecation: DeprecationStatus::Current,
                type_: FieldType::new("ID")
            },
        ];
        let base_field = SelectionField {
            name: "",
            arguments: Vec::new(),
            alias: None,
            fields: Selection(Vec::new())
        };
        let fragment = GqlFragment {
            name: "Fragment",
            is_required: Cell::new(true),
            on: FragmentTarget::Object(&fragment_on),
            selection: Selection(vec![
                SelectionItem::Field(SelectionField {
                    name: "field_1",
                    ..base_field.clone()
                }),
                SelectionItem::Field(SelectionField {
                    name: "field_2",
                    ..base_field
                }),
            ])
        };

        let selector: Vec<String> = get_fragment_selectors(&fragment, &schema_fields, &ctx)
            .into_iter()
            .map(|x| x.to_string())
            .collect();
        let expected = vec![
            quote!(::artemis::codegen::FieldSelector::Scalar(
                "field_1",
                String::new()
            ))
            .to_string(),
            quote!(::artemis::codegen::FieldSelector::Scalar(
                "field_2",
                String::new()
            ))
            .to_string(),
        ];

        assert_eq!(selector, expected);
    }

    #[test]
    fn nested_fragment_selector_returns_flattened_selectors() {
        let schema = crate::schema::Schema::new();
        let mut ctx = QueryContext::new_empty(&schema);
        let fragment_on = GqlObject {
            name: "Test",
            is_required: Cell::new(true),
            description: None,
            fields: Vec::new()
        };
        let schema_fields = vec![
            GqlObjectField {
                name: "field_1",
                description: None,
                deprecation: DeprecationStatus::Current,
                type_: FieldType::new("ID")
            },
            GqlObjectField {
                name: "field_2",
                description: None,
                deprecation: DeprecationStatus::Current,
                type_: FieldType::new("ID")
            },
            GqlObjectField {
                name: "field_3",
                description: None,
                deprecation: DeprecationStatus::Current,
                type_: FieldType::new("ID")
            },
            GqlObjectField {
                name: "field_test",
                description: None,
                deprecation: DeprecationStatus::Current,
                type_: FieldType::new("Test")
            },
        ];
        let base_field = SelectionField {
            name: "",
            arguments: Vec::new(),
            alias: None,
            fields: Selection(Vec::new())
        };
        let fragment = GqlFragment {
            name: "Fragment",
            is_required: Cell::new(true),
            on: FragmentTarget::Object(&fragment_on),
            selection: Selection(vec![
                SelectionItem::Field(SelectionField {
                    name: "field_1",
                    ..base_field.clone()
                }),
                SelectionItem::FragmentSpread(SelectionFragmentSpread {
                    fragment_name: "Fragment2"
                }),
            ])
        };
        let fragment2 = GqlFragment {
            name: "Fragment2",
            is_required: Cell::new(true),
            on: FragmentTarget::Object(&fragment_on),
            selection: Selection(vec![
                SelectionItem::Field(SelectionField {
                    name: "field_2",
                    ..base_field.clone()
                }),
                SelectionItem::Field(SelectionField {
                    name: "field_3",
                    ..base_field.clone()
                }),
            ])
        };

        ctx.fragments.insert("Fragment", fragment);
        ctx.fragments.insert("Fragment2", fragment2);

        let selector: Vec<String> =
            get_fragment_selectors(ctx.fragments.get("Fragment").unwrap(), &schema_fields, &ctx)
                .into_iter()
                .map(|x| x.to_string())
                .collect();
        let expected = vec![
            quote!(::artemis::codegen::FieldSelector::Scalar(
                "field_1",
                String::new()
            ))
            .to_string(),
            quote!(::artemis::codegen::FieldSelector::Scalar(
                "field_2",
                String::new()
            ))
            .to_string(),
            quote!(::artemis::codegen::FieldSelector::Scalar(
                "field_3",
                String::new()
            ))
            .to_string(),
        ];

        assert_eq!(selector, expected);
    }

    #[test]
    fn keyword_replace() {
        use super::keyword_replace;
        assert_eq!("fora", keyword_replace("fora"));
        assert_eq!("in_", keyword_replace("in"));
        assert_eq!("fn_", keyword_replace("fn"));
        assert_eq!("struct_", keyword_replace("struct"));
    }
}
