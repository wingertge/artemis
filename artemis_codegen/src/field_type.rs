use crate::{
    enums::ENUMS_PREFIX,
    introspection_response,
    query::QueryContext,
    schema::DEFAULT_SCALARS,
    shared::{ArgumentValue, ToRust}
};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

#[derive(Clone, Debug, PartialEq, Hash)]
enum GraphqlTypeQualifier {
    Required,
    List
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct FieldType<'a> {
    /// The type name of the field.
    ///
    /// e.g. for `[Int]!`, this would return `Int`.
    name: &'a str,
    /// An ordered list of qualifiers, from outer to inner.
    ///
    /// e.g. `[Int]!` would have `vec![List, Optional]`, but `[Int!]` would have `vec![Optional,
    /// List]`.
    qualifiers: Vec<GraphqlTypeQualifier>
}

impl<'a> FieldType<'a> {
    pub(crate) fn new(name: &'a str) -> Self {
        FieldType {
            name,
            qualifiers: Vec::new()
        }
    }

    #[cfg(test)]
    pub(crate) fn list(mut self) -> Self {
        self.qualifiers.insert(0, GraphqlTypeQualifier::List);
        self
    }

    #[cfg(test)]
    pub(crate) fn nonnull(mut self) -> Self {
        self.qualifiers.insert(0, GraphqlTypeQualifier::Required);
        self
    }

    pub(crate) fn args_as_string(args: Option<TokenStream>) -> TokenStream {
        if let Some(arguments) = args {
            quote!(String::from(#arguments))
        } else {
            quote!(String::new())
        }
    }

    pub(crate) fn to_typescript(&self, context: &QueryContext<'_, '_>, prefix: &str) -> String {
        let prefix: &str = if prefix.is_empty() {
            self.inner_name_str()
        } else {
            prefix
        };

        let full_name = {
            if context
                .schema
                .scalars
                .get(&self.name)
                .map(|s| s.is_required.set(true))
                .is_some()
                || DEFAULT_SCALARS.iter().any(|elem| elem == &self.name)
            {
                self.name.to_string()
            } else if context
                .schema
                .enums
                .get(&self.name)
                .map(|enm| enm.is_required.set(true))
                .is_some()
            {
                format!("{}{}", ENUMS_PREFIX, self.name)
            } else {
                if prefix.is_empty() {
                    panic!("Empty prefix for {:?}", self);
                }

                prefix.to_string()
            }
        };

        let mut qualified = full_name;

        let mut non_null = false;

        // Note: we iterate over qualifiers in reverse because it is more intuitive. This
        // means we start from the _inner_ type and make our way to the outside.
        for qualifier in self.qualifiers.iter().rev() {
            match (non_null, qualifier) {
                // We are in non-null context, and we wrap the non-null type into a list.
                // We switch back to null context.
                (true, GraphqlTypeQualifier::List) => {
                    qualified = format!("Array<{}>", qualified);
                    non_null = false;
                }
                // We are in nullable context, and we wrap the nullable type into a list.
                (false, GraphqlTypeQualifier::List) => {
                    qualified = format!("Array<Maybe<{}>>", qualified);
                }
                // We are in non-nullable context, but we can't double require a type
                // (!!).
                (true, GraphqlTypeQualifier::Required) => panic!("double required annotation"),
                // We are in nullable context, and we switch to non-nullable context.
                (false, GraphqlTypeQualifier::Required) => {
                    non_null = true;
                }
            }
        }

        // If we are in nullable context at the end of the iteration, we wrap the whole
        // type with an Option.
        if !non_null {
            qualified = format!("Maybe<{}>", qualified);
        }

        qualified
    }

    pub(crate) fn field_selector(
        &self,
        ctx: &QueryContext<'_, '_>,
        prefix: &str,
        field_name: &str,
        args: Vec<(String, ArgumentValue)>
    ) -> TokenStream {
        let args = Self::args_as_string(args.to_rust());

        if ctx.is_scalar(self.name) || ctx.is_enum(self.name) {
            quote! {
                ::artemis::codegen::FieldSelector::Scalar(#field_name, #args)
            }
        } else if ctx.is_union(self.name) {
            let type_ident = Ident::new(prefix, Span::call_site());
            let selection_fn = quote! { ::std::sync::Arc::new(|typename| #type_ident::selection(typename, variables)) };

            quote! {
                ::artemis::codegen::FieldSelector::Union(#field_name, #args, #selection_fn)
            }
        } else {
            let type_ident = Ident::new(prefix, Span::call_site());
            let typename = self.name;

            quote! {
                ::artemis::codegen::FieldSelector::Object(#field_name, #args, #typename, #type_ident::selection(variables))
            }
        }
    }

    /// Takes a field type with its name.
    pub(crate) fn to_rust(&self, context: &QueryContext<'_, '_>, prefix: &str) -> TokenStream {
        let prefix: &str = if prefix.is_empty() {
            self.inner_name_str()
        } else {
            prefix
        };

        let full_name = {
            if context.is_scalar(self.name) {
                self.name.to_string()
            } else if context.is_enum(self.name) {
                format!("{}{}", ENUMS_PREFIX, self.name)
            } else {
                if prefix.is_empty() {
                    panic!("Empty prefix for {:?}", self);
                }

                prefix.to_string()
            }
        };

        let norm = context.normalization;
        let full_name = norm.field_type(crate::shared::keyword_replace(&full_name));

        let full_name = Ident::new(&full_name, Span::call_site());
        let mut qualified = quote!(#full_name);

        let mut non_null = false;

        // Note: we iterate over qualifiers in reverse because it is more intuitive. This
        // means we start from the _inner_ type and make our way to the outside.
        for qualifier in self.qualifiers.iter().rev() {
            match (non_null, qualifier) {
                // We are in non-null context, and we wrap the non-null type into a list.
                // We switch back to null context.
                (true, GraphqlTypeQualifier::List) => {
                    qualified = quote!(Vec<#qualified>);
                    non_null = false;
                }
                // We are in nullable context, and we wrap the nullable type into a list.
                (false, GraphqlTypeQualifier::List) => {
                    qualified = quote!(Vec<Option<#qualified>>);
                }
                // We are in non-nullable context, but we can't double require a type
                // (!!).
                (true, GraphqlTypeQualifier::Required) => panic!("double required annotation"),
                // We are in nullable context, and we switch to non-nullable context.
                (false, GraphqlTypeQualifier::Required) => {
                    non_null = true;
                }
            }
        }

        // If we are in nullable context at the end of the iteration, we wrap the whole
        // type with an Option.
        if !non_null {
            qualified = quote!(Option<#qualified>);
        }

        qualified
    }

    /// Return the innermost name - we mostly use this for looking types up in our Schema struct.
    pub fn inner_name_str(&self) -> &str {
        self.name
    }

    /// Is the type nullable?
    ///
    /// Note: a list of nullable values is considered nullable only if the list itself is nullable.
    pub fn is_optional(&self) -> bool {
        if let Some(qualifier) = self.qualifiers.get(0) {
            qualifier != &GraphqlTypeQualifier::Required
        } else {
            true
        }
    }

    /// A type is indirected if it is a (flat or nested) list type, optional or not.
    ///
    /// We use this to determine whether a type needs to be boxed for recursion.
    pub fn is_indirected(&self) -> bool {
        self.qualifiers
            .iter()
            .any(|qualifier| qualifier == &GraphqlTypeQualifier::List)
    }
}

impl<'schema> std::convert::From<&'schema graphql_parser::schema::Type> for FieldType<'schema> {
    fn from(schema_type: &'schema graphql_parser::schema::Type) -> FieldType<'schema> {
        from_schema_type_inner(schema_type)
    }
}

fn graphql_parser_depth(schema_type: &graphql_parser::schema::Type) -> usize {
    match schema_type {
        graphql_parser::schema::Type::ListType(inner) => 1 + graphql_parser_depth(inner),
        graphql_parser::schema::Type::NonNullType(inner) => 1 + graphql_parser_depth(inner),
        graphql_parser::schema::Type::NamedType(_) => 0
    }
}

fn from_schema_type_inner(inner: &graphql_parser::schema::Type) -> FieldType<'_> {
    use graphql_parser::schema::Type::*;

    let qualifiers_depth = graphql_parser_depth(inner);
    let mut qualifiers = Vec::with_capacity(qualifiers_depth);

    let mut inner = inner;

    loop {
        match inner {
            ListType(new_inner) => {
                qualifiers.push(GraphqlTypeQualifier::List);
                inner = new_inner;
            }
            NonNullType(new_inner) => {
                qualifiers.push(GraphqlTypeQualifier::Required);
                inner = new_inner;
            }
            NamedType(name) => return FieldType { name, qualifiers }
        }
    }
}

fn json_type_qualifiers_depth(typeref: &introspection_response::TypeRef) -> usize {
    use crate::introspection_response::*;

    match (typeref.kind.as_ref(), typeref.of_type.as_ref()) {
        (Some(__TypeKind::NON_NULL), Some(inner)) => 1 + json_type_qualifiers_depth(inner),
        (Some(__TypeKind::LIST), Some(inner)) => 1 + json_type_qualifiers_depth(inner),
        (Some(_), None) => 0,
        _ => panic!("Non-convertible type in JSON schema: {:?}", typeref)
    }
}

fn from_json_type_inner(inner: &introspection_response::TypeRef) -> FieldType<'_> {
    use crate::introspection_response::*;

    let qualifiers_depth = json_type_qualifiers_depth(inner);
    let mut qualifiers = Vec::with_capacity(qualifiers_depth);

    let mut inner = inner;

    loop {
        match (
            inner.kind.as_ref(),
            inner.of_type.as_ref(),
            inner.name.as_ref()
        ) {
            (Some(__TypeKind::NON_NULL), Some(new_inner), _) => {
                qualifiers.push(GraphqlTypeQualifier::Required);
                inner = &new_inner;
            }
            (Some(__TypeKind::LIST), Some(new_inner), _) => {
                qualifiers.push(GraphqlTypeQualifier::List);
                inner = &new_inner;
            }
            (Some(_), None, Some(name)) => return FieldType { name, qualifiers },
            _ => panic!("Non-convertible type in JSON schema: {:?}", inner)
        }
    }
}

impl<'schema> std::convert::From<&'schema introspection_response::FullTypeFieldsType>
    for FieldType<'schema>
{
    fn from(
        schema_type: &'schema introspection_response::FullTypeFieldsType
    ) -> FieldType<'schema> {
        from_json_type_inner(&schema_type.type_ref)
    }
}

impl<'a> std::convert::From<&'a introspection_response::InputValueType> for FieldType<'a> {
    fn from(schema_type: &'a introspection_response::InputValueType) -> FieldType<'a> {
        from_json_type_inner(&schema_type.type_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        enums::GqlEnum,
        introspection_response::{FullTypeFieldsType, TypeRef, __TypeKind},
        objects::GqlObject,
        unions::GqlUnion
    };
    use graphql_parser::schema::Type as GqlParserType;
    use std::cell::Cell;

    fn schema() -> crate::schema::Schema<'static> {
        let mut schema = crate::schema::Schema::new();
        schema.objects.insert(
            "Cat",
            GqlObject {
                name: "Cat",
                description: None,
                fields: Vec::new(),
                is_required: Cell::new(true)
            }
        );
        schema.scalars.insert(
            "Age",
            crate::scalars::Scalar {
                name: "Age",
                is_required: Cell::new(true),
                description: None
            }
        );
        schema.enums.insert(
            "Animal",
            GqlEnum {
                name: "Animal",
                description: None,
                is_required: Cell::new(true),
                variants: Vec::new()
            }
        );
        schema.unions.insert(
            "Union",
            GqlUnion {
                name: "Union",
                description: None,
                is_required: Cell::new(true),
                variants: vec!["Variant1", "Variant2"].into_iter().collect()
            }
        );
        schema
    }

    fn with_ctx<F>(f: F)
    where
        F: FnOnce(&QueryContext<'_, '_>)
    {
        let schema = schema();
        let ctx = QueryContext::new_empty(&schema);
        f(&ctx);
    }

    /// ToRust Tests
    mod to_rust {
        use super::with_ctx;
        use crate::field_type::{FieldType, GraphqlTypeQualifier};
        use quote::quote;

        #[test]
        fn non_null_type_produces_raw_typename() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Cat").nonnull();

                assert_eq!(ty.to_rust(ctx, "Cat").to_string(), quote!(Cat).to_string());
            });
        }

        #[test]
        fn nullable_type_produces_option() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Cat");

                assert_eq!(
                    ty.to_rust(ctx, "Cat").to_string(),
                    quote!(Option<Cat>).to_string()
                );
            });
        }

        #[test]
        fn scalar_type_produces_raw_typename() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Age").nonnull();

                assert_eq!(ty.to_rust(ctx, "").to_string(), quote!(Age).to_string());
            })
        }

        #[test]
        fn enum_type_produces_raw_typename() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Animal").nonnull();

                assert_eq!(ty.to_rust(ctx, "").to_string(), quote!(Animal).to_string());
            })
        }

        #[test]
        fn list_produces_vec() {
            with_ctx(|ctx| {
                let mut ty = FieldType::new("Cat").nonnull();
                ty.qualifiers.push(GraphqlTypeQualifier::List);
                ty.qualifiers.push(GraphqlTypeQualifier::Required);

                assert_eq!(
                    ty.to_rust(ctx, "").to_string(),
                    quote!(Vec<Cat>).to_string()
                );
            })
        }

        #[test]
        fn list_of_options_produces_vec_of_option() {
            with_ctx(|ctx| {
                let mut ty = FieldType::new("Cat").nonnull();
                ty.qualifiers.push(GraphqlTypeQualifier::List);

                assert_eq!(
                    ty.to_rust(ctx, "").to_string(),
                    quote!(Vec<Option<Cat>>).to_string()
                );
            })
        }
    }

    /// FieldSelector Tests
    mod field_selector {
        use crate::field_type::{tests::with_ctx, FieldType};
        use quote::quote;

        #[test]
        fn object_produces_object_field_selector() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Cat").nonnull();

                let selector = ty.field_selector(ctx, "Cat", "cat_field", Vec::new());
                let expected = quote! {
                    ::artemis::codegen::FieldSelector::Object(
                        "cat_field",
                        String::new(),
                        "Cat",
                        Cat::selection(variables)
                    )
                };

                assert_eq!(selector.to_string(), expected.to_string())
            })
        }

        #[test]
        fn scalar_produces_scalar_field_selector() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Age").nonnull();

                let selector = ty.field_selector(ctx, "Age", "age_field", Vec::new());
                let expected = quote! {
                    ::artemis::codegen::FieldSelector::Scalar("age_field", String::new())
                };

                assert_eq!(selector.to_string(), expected.to_string())
            })
        }

        #[test]
        fn enum_produces_scalar_field_selector() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Animal").nonnull();

                let selector = ty.field_selector(ctx, "Animal", "animal_field", Vec::new());
                let expected = quote! {
                    ::artemis::codegen::FieldSelector::Scalar("animal_field", String::new())
                };

                assert_eq!(selector.to_string(), expected.to_string())
            })
        }

        #[test]
        fn union_produces_union_field_selector() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Union").nonnull();

                let selector = ty.field_selector(ctx, "Union", "union_field", Vec::new());
                let expected = quote! {
                    ::artemis::codegen::FieldSelector::Union(
                        "union_field",
                        String::new(),
                        ::std::sync::Arc::new(|typename| Union::selection(typename, variables))
                    )
                };

                assert_eq!(selector.to_string(), expected.to_string())
            })
        }
    }

    /// Typescript Tests
    mod typescript {
        use super::with_ctx;
        use crate::field_type::{FieldType, GraphqlTypeQualifier};

        #[test]
        fn non_null_type_produces_typescript_typename() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Cat").nonnull();

                assert_eq!(ty.to_typescript(ctx, "Cat"), "Cat");
            });
        }

        #[test]
        fn nullable_type_produces_maybe() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Cat");

                assert_eq!(ty.to_typescript(ctx, "Cat"), "Maybe<Cat>");
            });
        }

        #[test]
        fn scalar_type_produces_typescript_typename() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Age").nonnull();

                assert_eq!(ty.to_typescript(ctx, ""), "Age");
            })
        }

        #[test]
        fn enum_type_produces_typescript_typename() {
            with_ctx(|ctx| {
                let ty = FieldType::new("Animal").nonnull();

                assert_eq!(ty.to_typescript(ctx, ""), "Animal");
            })
        }

        #[test]
        fn list_produces_array() {
            with_ctx(|ctx| {
                let mut ty = FieldType::new("Cat").nonnull();
                ty.qualifiers.push(GraphqlTypeQualifier::List);
                ty.qualifiers.push(GraphqlTypeQualifier::Required);

                assert_eq!(ty.to_typescript(ctx, ""), "Array<Cat>");
            })
        }

        #[test]
        fn list_of_options_produces_array_of_maybe() {
            with_ctx(|ctx| {
                let mut ty = FieldType::new("Cat").nonnull();
                ty.qualifiers.push(GraphqlTypeQualifier::List);

                assert_eq!(ty.to_typescript(ctx, ""), "Array<Maybe<Cat>>");
            })
        }
    }

    // TODO: These tests are awful, replace them ASAP
    #[test]
    fn field_type_from_graphql_parser_schema_type_works() {
        let ty = GqlParserType::NamedType("Cat".to_owned());
        assert_eq!(FieldType::from(&ty), FieldType::new("Cat"));

        let ty = GqlParserType::NonNullType(Box::new(GqlParserType::NamedType("Cat".to_owned())));

        assert_eq!(FieldType::from(&ty), FieldType::new("Cat").nonnull());
    }

    // TODO: These tests are awful, replace them ASAP
    #[test]
    fn field_type_from_introspection_response_works() {
        let ty = FullTypeFieldsType {
            type_ref: TypeRef {
                kind: Some(__TypeKind::OBJECT),
                name: Some("Cat".into()),
                of_type: None
            }
        };
        assert_eq!(FieldType::from(&ty), FieldType::new("Cat"));

        let ty = FullTypeFieldsType {
            type_ref: TypeRef {
                kind: Some(__TypeKind::NON_NULL),
                name: None,
                of_type: Some(Box::new(TypeRef {
                    kind: Some(__TypeKind::OBJECT),
                    name: Some("Cat".into()),
                    of_type: None
                }))
            }
        };
        assert_eq!(FieldType::from(&ty), FieldType::new("Cat").nonnull());
    }
}
