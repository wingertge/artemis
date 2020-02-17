use crate::{
    deprecation::DeprecationStrategy, fragments::GqlFragment, normalization::Normalization,
    schema::Schema, selection::Selection, CodegenError
};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use syn::Ident;

/// This holds all the information we need during the code generation phase.
pub(crate) struct QueryContext<'query, 'schema: 'query> {
    pub fragments: BTreeMap<&'query str, GqlFragment<'query>>,
    pub schema: &'schema Schema<'schema>,
    pub deprecation_strategy: DeprecationStrategy,
    pub normalization: Normalization,
    pub include_query_info: bool,
    variables_derives: Vec<Ident>,
    response_derives: Vec<Ident>
}

impl<'query, 'schema> QueryContext<'query, 'schema> {
    /// Create a QueryContext with the given Schema.
    pub(crate) fn new(
        schema: &'schema Schema<'schema>,
        deprecation_strategy: DeprecationStrategy,
        normalization: Normalization,
        include_query_info: bool
    ) -> QueryContext<'query, 'schema> {
        QueryContext {
            fragments: BTreeMap::new(),
            schema,
            deprecation_strategy,
            normalization,
            include_query_info,
            variables_derives: vec![
                Ident::new("Serialize", Span::call_site()),
                Ident::new("Clone", Span::call_site()),
            ],
            response_derives: vec![
                Ident::new("Deserialize", Span::call_site()),
                Ident::new("Clone", Span::call_site()),
            ]
        }
    }

    /// Mark a fragment as required, so code is actually generated for it.
    pub(crate) fn require_fragment(&self, typename_: &str) {
        if let Some(fragment) = self.fragments.get(typename_) {
            fragment.require(&self);
        }
    }

    /// For testing only. creates an empty QueryContext with an empty Schema.
    #[cfg(test)]
    pub(crate) fn new_empty(schema: &'schema Schema<'_>) -> QueryContext<'query, 'schema> {
        QueryContext {
            fragments: BTreeMap::new(),
            schema,
            deprecation_strategy: DeprecationStrategy::Allow,
            normalization: Normalization::None,
            include_query_info: true,
            variables_derives: vec![
                Ident::new("Serialize", Span::call_site()),
                Ident::new("Clone", Span::call_site()),
            ],
            response_derives: vec![
                Ident::new("Deserialize", Span::call_site()),
                Ident::new("Clone", Span::call_site()),
            ]
        }
    }

    /// Expand the deserialization data structures for the given field.
    pub(crate) fn maybe_expand_field(
        &self,
        ty: &str,
        selection: &Selection<'_>,
        prefix: &str
    ) -> Result<Option<(TokenStream, HashSet<String>)>, CodegenError> {
        if self.schema.contains_scalar(ty) {
            Ok(None)
        } else if let Some(enm) = self.schema.enums.get(ty) {
            enm.is_required.set(true);
            Ok(None) // we already expand enums separately
        } else if let Some(obj) = self.schema.objects.get(ty) {
            obj.is_required.set(true);
            obj.response_for_selection(self, &selection, prefix)
                .map(|(tokens, mut types)| {
                    types.insert(ty.to_string());
                    Some((tokens, types))
                })
                .map_err(Into::into)
        } else if let Some(iface) = self.schema.interfaces.get(ty) {
            iface.is_required.set(true);
            iface
                .response_for_selection(self, &selection, prefix)
                .map(|(tokens, mut types)| {
                    types.insert(ty.to_string());
                    Some((tokens, types))
                })
                .map_err(Into::into)
        } else if let Some(unn) = self.schema.unions.get(ty) {
            unn.is_required.set(true);
            unn.response_for_selection(self, &selection, prefix)
                .map(|(tokens, mut types)| {
                    types.insert(ty.to_string());
                    Some((tokens, types))
                })
                .map_err(Into::into)
        } else {
            Err(CodegenError::TypeError(format!("Unknown type: {}", ty)))
        }
    }

    pub(crate) fn ingest_response_derives(
        &mut self,
        attribute_value: &str
    ) -> Result<(), CodegenError> {
        if self.response_derives.len() > 2 {
            return Err(CodegenError::InternalError(format!(
                "ingest_response_derives should only be called once"
            )));
        }

        self.response_derives.extend(
            attribute_value
                .split(',')
                .map(str::trim)
                .map(|s| Ident::new(s, Span::call_site()))
        );
        Ok(())
    }

    pub(crate) fn ingest_variables_derives(
        &mut self,
        attribute_value: &str
    ) -> Result<(), CodegenError> {
        if self.variables_derives.len() > 2 {
            return Err(CodegenError::InternalError(format!(
                "ingest_variables_derives should only be called once"
            )));
        }

        self.variables_derives.extend(
            attribute_value
                .split(',')
                .map(str::trim)
                .map(|s| Ident::new(s, Span::call_site()))
        );
        Ok(())
    }

    pub(crate) fn variables_derives(&self) -> TokenStream {
        let derives: BTreeSet<&Ident> = self.variables_derives.iter().collect();
        let derives = derives.iter();

        quote! {
            #[derive( #(#derives),* )]
        }
    }

    pub(crate) fn response_derives(&self) -> TokenStream {
        let derives: BTreeSet<&Ident> = self.response_derives.iter().collect();
        let derives = derives.iter();
        quote! {
            #[derive( #(#derives),* )]
        }
    }

    pub(crate) fn response_enum_derives(&self) -> TokenStream {
        let always_derives = [
            Ident::new("Eq", Span::call_site()),
            Ident::new("PartialEq", Span::call_site())
        ];
        let mut enum_derives: BTreeSet<_> = self
            .response_derives
            .iter()
            .filter(|derive| {
                // Do not apply the "Default" derive to enums.
                let derive = derive.to_string();
                derive != "Serialize"
                    && derive != "Deserialize"
                    && derive != "Default"
                    && derive != "Clone"
            })
            .collect();
        enum_derives.extend(always_derives.iter());
        quote! {
            #[derive( #(#enum_derives),* )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_derives_ingestion_works() {
        let schema = crate::schema::Schema::new();
        let mut context = QueryContext::new_empty(&schema);

        context
            .ingest_response_derives("PartialEq, PartialOrd, Serialize")
            .unwrap();

        assert_eq!(
            context.response_derives().to_string(),
            "# [ derive ( Clone , Deserialize , PartialEq , PartialOrd , Serialize ) ]"
        );
    }

    #[test]
    fn response_enum_derives_does_not_produce_empty_list() {
        let schema = crate::schema::Schema::new();
        let context = QueryContext::new_empty(&schema);
        assert_eq!(
            context.response_enum_derives().to_string(),
            "# [ derive ( Eq , PartialEq ) ]"
        );
    }

    #[test]
    fn response_enum_derives_works() {
        let schema = crate::schema::Schema::new();
        let mut context = QueryContext::new_empty(&schema);

        context
            .ingest_response_derives("PartialEq, PartialOrd, Serialize")
            .unwrap();

        assert_eq!(
            context.response_enum_derives().to_string(),
            "# [ derive ( Eq , PartialEq , PartialOrd ) ]"
        );
    }

    #[test]
    fn response_derives_fails_when_called_twice() {
        let schema = crate::schema::Schema::new();
        let mut context = QueryContext::new_empty(&schema);

        assert!(context
            .ingest_response_derives("PartialEq, PartialOrd")
            .is_ok());
        assert!(context.ingest_response_derives("Serialize").is_err());
    }
}
