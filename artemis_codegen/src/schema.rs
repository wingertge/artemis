use crate::{
    deprecation::DeprecationStatus,
    enums::{EnumVariant, GqlEnum},
    field_type::FieldType,
    inputs::GqlInput,
    interfaces::GqlInterface,
    introspection_response,
    objects::{GqlObject, GqlObjectField},
    scalars::Scalar,
    unions::GqlUnion,
    CodegenError
};
use graphql_parser::{self, schema};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const DEFAULT_SCALARS: &[&str] = &["ID", "String", "Int", "Float", "Boolean"];

/// Intermediate representation for a parsed GraphQL schema used during code generation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Schema<'schema> {
    pub(crate) enums: BTreeMap<&'schema str, GqlEnum<'schema>>,
    pub(crate) inputs: BTreeMap<&'schema str, GqlInput<'schema>>,
    pub(crate) interfaces: BTreeMap<&'schema str, GqlInterface<'schema>>,
    pub(crate) objects: BTreeMap<&'schema str, GqlObject<'schema>>,
    pub(crate) scalars: BTreeMap<&'schema str, Scalar<'schema>>,
    pub(crate) unions: BTreeMap<&'schema str, GqlUnion<'schema>>,
    pub(crate) query_type: Option<&'schema str>,
    pub(crate) mutation_type: Option<&'schema str>,
    pub(crate) subscription_type: Option<&'schema str>
}

impl<'schema> Schema<'schema> {
    pub(crate) fn new() -> Schema<'schema> {
        Schema {
            enums: BTreeMap::new(),
            inputs: BTreeMap::new(),
            interfaces: BTreeMap::new(),
            objects: BTreeMap::new(),
            scalars: BTreeMap::new(),
            unions: BTreeMap::new(),
            query_type: None,
            mutation_type: None,
            subscription_type: None
        }
    }

    pub(crate) fn ingest_interface_implementations(
        &mut self,
        impls: BTreeMap<&'schema str, Vec<&'schema str>>
    ) -> Result<(), CodegenError> {
        impls
            .into_iter()
            .map(|(iface_name, implementors)| {
                let iface = self.interfaces.get_mut(&iface_name).ok_or_else(|| {
                    CodegenError::TypeError(format!("interface not found: {}", iface_name))
                })?;
                iface.implemented_by = implementors.iter().cloned().collect();
                Ok(())
            })
            .collect()
    }

    pub(crate) fn require(&self, typename_: &str) {
        DEFAULT_SCALARS
            .iter()
            .find(|&&s| s == typename_)
            .map(|_| ())
            .or_else(|| {
                self.enums
                    .get(typename_)
                    .map(|enm| enm.is_required.set(true))
            })
            .or_else(|| self.inputs.get(typename_).map(|input| input.require(self)))
            .or_else(|| {
                self.objects
                    .get(typename_)
                    .map(|object| object.require(self))
            })
            .or_else(|| {
                self.scalars
                    .get(typename_)
                    .map(|scalar| scalar.is_required.set(true))
            });
    }

    pub(crate) fn contains_scalar(&self, type_name: &str) -> bool {
        DEFAULT_SCALARS.iter().any(|s| s == &type_name) || self.scalars.contains_key(type_name)
    }

    pub(crate) fn fragment_target(
        &self,
        target_name: &str
    ) -> Option<crate::fragments::FragmentTarget<'_>> {
        self.objects
            .get(target_name)
            .map(crate::fragments::FragmentTarget::Object)
            .or_else(|| {
                self.interfaces
                    .get(target_name)
                    .map(crate::fragments::FragmentTarget::Interface)
            })
            .or_else(|| {
                self.unions
                    .get(target_name)
                    .map(crate::fragments::FragmentTarget::Union)
            })
    }
}

impl<'schema> std::convert::From<&'schema graphql_parser::schema::Document> for Schema<'schema> {
    fn from(ast: &'schema graphql_parser::schema::Document) -> Schema<'schema> {
        let mut schema = Schema::new();

        // Holds which objects implement which interfaces so we can populate GqlInterface#implemented_by later.
        // It maps interface names to a vec of implementation names.
        let mut interface_implementations: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

        for definition in &ast.definitions {
            match definition {
                schema::Definition::TypeDefinition(ty_definition) => match ty_definition {
                    schema::TypeDefinition::Object(obj) => {
                        for implementing in &obj.implements_interfaces {
                            let name = &obj.name;
                            interface_implementations
                                .entry(implementing)
                                .and_modify(|objects| objects.push(name))
                                .or_insert_with(|| vec![name]);
                        }

                        schema
                            .objects
                            .insert(&obj.name, GqlObject::from_graphql_parser_object(&obj));
                    }
                    schema::TypeDefinition::Enum(enm) => {
                        schema.enums.insert(
                            &enm.name,
                            GqlEnum {
                                name: &enm.name,
                                description: enm.description.as_deref(),
                                variants: enm
                                    .values
                                    .iter()
                                    .map(|v| EnumVariant {
                                        description: v.description.as_deref(),
                                        name: &v.name
                                    })
                                    .collect(),
                                is_required: false.into()
                            }
                        );
                    }
                    schema::TypeDefinition::Scalar(scalar) => {
                        schema.scalars.insert(
                            &scalar.name,
                            Scalar {
                                name: &scalar.name,
                                description: scalar.description.as_deref(),
                                is_required: false.into()
                            }
                        );
                    }
                    schema::TypeDefinition::Union(union) => {
                        let variants: BTreeSet<&str> =
                            union.types.iter().map(String::as_str).collect();
                        schema.unions.insert(
                            &union.name,
                            GqlUnion {
                                name: &union.name,
                                variants,
                                description: union.description.as_deref(),
                                is_required: false.into()
                            }
                        );
                    }
                    schema::TypeDefinition::Interface(interface) => {
                        let mut iface =
                            GqlInterface::new(&interface.name, interface.description.as_deref());
                        iface
                            .fields
                            .extend(interface.fields.iter().map(|f| GqlObjectField {
                                description: f.description.as_deref(),
                                name: f.name.as_str(),
                                type_: FieldType::from(&f.field_type),
                                deprecation: DeprecationStatus::Current
                            }));
                        schema.interfaces.insert(&interface.name, iface);
                    }
                    schema::TypeDefinition::InputObject(input) => {
                        schema.inputs.insert(&input.name, GqlInput::from(input));
                    }
                },
                schema::Definition::DirectiveDefinition(_) => (),
                schema::Definition::TypeExtension(_extension) => (),
                schema::Definition::SchemaDefinition(definition) => {
                    schema.query_type = definition.query.as_deref();
                    schema.mutation_type = definition.mutation.as_deref();
                    schema.subscription_type = definition.subscription.as_deref();
                }
            }
        }

        schema
            .ingest_interface_implementations(interface_implementations)
            .expect("schema ingestion");

        schema
    }
}

impl<'schema> std::convert::From<&'schema crate::introspection_response::IntrospectionResponse>
    for Schema<'schema>
{
    fn from(src: &'schema crate::introspection_response::IntrospectionResponse) -> Self {
        use crate::introspection_response::__TypeKind;

        let mut schema = Schema::new();
        let root = src
            .as_schema()
            .schema
            .as_ref()
            .expect("__schema is not null");

        schema.query_type = root
            .query_type
            .as_ref()
            .and_then(|ty| ty.name.as_ref())
            .map(String::as_str);
        schema.mutation_type = root
            .mutation_type
            .as_ref()
            .and_then(|ty| ty.name.as_ref())
            .map(String::as_str);
        schema.subscription_type = root
            .subscription_type
            .as_ref()
            .and_then(|ty| ty.name.as_ref())
            .map(String::as_str);

        // Holds which objects implement which interfaces so we can populate GqlInterface#implemented_by later.
        // It maps interface names to a vec of implementation names.
        let mut interface_implementations: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

        for ty in root
            .types
            .as_ref()
            .expect("types in schema")
            .iter()
            .filter_map(|t| t.as_ref().map(|t| &t.full_type))
        {
            let name: &str = ty.name.as_deref().expect("type definition name");

            match ty.kind {
                Some(__TypeKind::ENUM) => {
                    let variants: Vec<EnumVariant<'_>> = ty
                        .enum_values
                        .as_ref()
                        .expect("enum variants")
                        .iter()
                        .map(|t| {
                            t.as_ref().map(|t| EnumVariant {
                                description: t.description.as_deref(),
                                name: t.name.as_deref().expect("enum variant name")
                            })
                        })
                        .filter_map(|t| t)
                        .collect();
                    let enm = GqlEnum {
                        name,
                        description: ty.description.as_deref(),
                        variants,
                        is_required: false.into()
                    };
                    schema.enums.insert(name, enm);
                }
                Some(__TypeKind::SCALAR) => {
                    if DEFAULT_SCALARS.iter().find(|s| s == &&name).is_none() {
                        schema.scalars.insert(
                            name,
                            Scalar {
                                name,
                                description: ty.description.as_deref(),
                                is_required: false.into()
                            }
                        );
                    }
                }
                Some(__TypeKind::UNION) => {
                    let variants: BTreeSet<&str> = ty
                        .possible_types
                        .as_ref()
                        .unwrap()
                        .iter()
                        .filter_map(|t| t.as_ref().and_then(|t| t.type_ref.name.as_deref()))
                        .collect();
                    schema.unions.insert(
                        name,
                        GqlUnion {
                            name: ty.name.as_deref().expect("unnamed union"),
                            description: ty.description.as_deref(),
                            variants,
                            is_required: false.into()
                        }
                    );
                }
                Some(__TypeKind::OBJECT) => {
                    for implementing in ty
                        .interfaces
                        .as_deref()
                        .unwrap_or_else(|| &[])
                        .iter()
                        .filter_map(Option::as_ref)
                        .map(|t| &t.type_ref.name)
                    {
                        interface_implementations
                            .entry(
                                implementing
                                    .as_ref()
                                    .map(String::as_str)
                                    .expect("interface name")
                            )
                            .and_modify(|objects| objects.push(name))
                            .or_insert_with(|| vec![name]);
                    }

                    schema
                        .objects
                        .insert(name, GqlObject::from_introspected_schema_json(ty));
                }
                Some(__TypeKind::INTERFACE) => {
                    let mut iface = GqlInterface::new(name, ty.description.as_deref());
                    iface.fields.extend(
                        ty.fields
                            .as_ref()
                            .expect("interface fields")
                            .iter()
                            .filter_map(Option::as_ref)
                            .map(|f| GqlObjectField {
                                description: f.description.as_deref(),
                                name: f.name.as_ref().expect("field name").as_str(),
                                type_: FieldType::from(f.type_.as_ref().expect("field type")),
                                deprecation: DeprecationStatus::Current
                            })
                    );
                    schema.interfaces.insert(name, iface);
                }
                Some(__TypeKind::INPUT_OBJECT) => {
                    schema.inputs.insert(name, GqlInput::from(ty));
                }
                _ => unimplemented!("unimplemented definition")
            }
        }

        schema
            .ingest_interface_implementations(interface_implementations)
            .expect("schema ingestion");

        schema
    }
}

pub(crate) enum ParsedSchema {
    GraphQLParser(graphql_parser::schema::Document),
    Json(introspection_response::IntrospectionResponse)
}

impl<'schema> From<&'schema ParsedSchema> for Schema<'schema> {
    fn from(parsed_schema: &'schema ParsedSchema) -> Schema<'schema> {
        match parsed_schema {
            ParsedSchema::GraphQLParser(s) => s.into(),
            ParsedSchema::Json(s) => s.into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::*;

    #[test]
    fn build_schema_works() {
        let gql_schema = include_str!("tests/star_wars_schema.graphql");
        let gql_schema = graphql_parser::parse_schema(gql_schema).unwrap();
        let built = Schema::from(&gql_schema);
        assert_eq!(
            built.objects.get("Droid"),
            Some(&GqlObject {
                description: None,
                name: "Droid",
                fields: vec![
                    GqlObjectField {
                        description: None,
                        name: TYPENAME_FIELD,
                        type_: FieldType::new(string_type()),
                        deprecation: DeprecationStatus::Current
                    },
                    GqlObjectField {
                        description: None,
                        name: "id",
                        type_: FieldType::new("ID").nonnull(),
                        deprecation: DeprecationStatus::Current
                    },
                    GqlObjectField {
                        description: None,
                        name: "name",
                        type_: FieldType::new("String").nonnull(),
                        deprecation: DeprecationStatus::Current
                    },
                    GqlObjectField {
                        description: None,
                        name: "friends",
                        type_: FieldType::new("Character").list(),
                        deprecation: DeprecationStatus::Current
                    },
                    GqlObjectField {
                        description: None,
                        name: "friendsConnection",
                        type_: FieldType::new("FriendsConnection").nonnull(),
                        deprecation: DeprecationStatus::Current
                    },
                    GqlObjectField {
                        description: None,
                        name: "appearsIn",
                        type_: FieldType::new("Episode").list().nonnull(),
                        deprecation: DeprecationStatus::Current
                    },
                    GqlObjectField {
                        description: None,
                        name: "primaryFunction",
                        type_: FieldType::new("String"),
                        deprecation: DeprecationStatus::Current
                    },
                ],
                is_required: false.into()
            })
        )
    }
}
