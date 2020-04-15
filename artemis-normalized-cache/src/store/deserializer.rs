use crate::store::data::{FieldKey, InMemoryData, Link, RefFieldKey};
use artemis::codegen::FieldSelector;
use flurry::epoch::Guard;
use serde::{
    de,
    de::{
        DeserializeSeed, EnumAccess, Error, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
        Visitor
    },
    forward_to_deserialize_any, Deserializer
};
use serde_json::Value;
use std::{fmt, fmt::Display};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct SerializerError {
    err: Box<ErrorImpl>
}

#[derive(Debug)]
enum ErrorImpl {
    Missing,
    Error(String),
    SerdeError(serde_json::Error)
}

impl From<serde_json::Error> for SerializerError {
    fn from(e: serde_json::Error) -> Self {
        Self {
            err: Box::new(ErrorImpl::SerdeError(e))
        }
    }
}

fn make_error(msg: String) -> SerializerError {
    SerializerError {
        err: Box::new(ErrorImpl::Error(msg))
    }
}

impl SerializerError {
    pub fn is_missing(&self) -> bool {
        match &*self.err {
            ErrorImpl::Missing => true,
            _ => false
        }
    }

    fn missing() -> Self {
        SerializerError {
            err: Box::new(ErrorImpl::Missing)
        }
    }
}

impl serde::de::Error for SerializerError {
    #[cold]
    fn custom<T: Display>(msg: T) -> SerializerError {
        make_error(msg.to_string())
    }

    #[cold]
    fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
        if let de::Unexpected::Unit = unexp {
            SerializerError::custom(format_args!("invalid type: null, expected {}", exp))
        } else {
            SerializerError::custom(format_args!("invalid type: {}, expected {}", unexp, exp))
        }
    }
}

impl serde::de::StdError for SerializerError {}
impl fmt::Display for SerializerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.err {
            ErrorImpl::Missing => write!(f, "missing"),
            ErrorImpl::Error(msg) => write!(f, "{}", msg),
            ErrorImpl::SerdeError(e) => write!(f, "{}", e)
        }
    }
}

const TYPENAME: FieldKey = FieldKey("__typename", String::new());

#[inline]
fn field_key<'a>(field_name: &'static str, args: &'a String) -> RefFieldKey<'a> {
    RefFieldKey(field_name, args)
}

#[inline]
fn selector_field_name(selector: &FieldSelector) -> &str {
    match selector {
        FieldSelector::Scalar(field_name, _) => *field_name,
        FieldSelector::Union(field_name, _, _) => *field_name,
        FieldSelector::Object(field_name, _, _, _) => *field_name
    }
}

struct SelectorDeserializer<'a, 'de> {
    data: &'de InMemoryData,
    guard: &'de Guard,
    selector: &'a FieldSelector,
    entity_key: &'a str,
    dependencies: *mut Vec<String>
}

impl<'a, 'de> Deserializer<'de> for SelectorDeserializer<'a, 'de> {
    type Error = SerializerError;

    #[inline]
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.selector {
            FieldSelector::Scalar(field_name, args) => {
                let scalar = self
                    .data
                    .read_record(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                Ok(scalar.deserialize_any(visitor)?)
            }
            FieldSelector::Object(field_name, args, _, inner_selection) => {
                let link = self
                    .data
                    .read_link(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                match link {
                    Link::Null => visitor.visit_unit(),
                    Link::Single(key) => visit_object(
                        self.data,
                        key,
                        inner_selection,
                        visitor,
                        self.guard,
                        self.dependencies
                    ),
                    Link::List(keys) => visit_array(
                        self.data,
                        keys,
                        inner_selection,
                        visitor,
                        self.guard,
                        self.dependencies
                    )
                }
            }
            FieldSelector::Union(field_name, args, inner_selection) => {
                let link = self
                    .data
                    .read_link(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                match link {
                    Link::Null => visitor.visit_unit(),
                    Link::Single(key) => {
                        let typename = self
                            .data
                            .read_record(&key, (&TYPENAME).into(), self.guard)
                            .ok_or_else(|| SerializerError::missing())?;
                        let typename = typename
                            .as_str()
                            .ok_or_else(|| SerializerError::custom("typename isn't a string"))?;
                        let inner_selection = inner_selection(typename);
                        visit_object(
                            self.data,
                            &key,
                            &inner_selection,
                            visitor,
                            self.guard,
                            self.dependencies
                        )
                    }
                    Link::List(keys) => visit_union_array(
                        self.data,
                        keys,
                        &**inner_selection,
                        visitor,
                        self.guard,
                        self.dependencies
                    )
                }
            }
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        match self.selector {
            FieldSelector::Scalar(field_name, args) => {
                let value = self
                    .data
                    .read_record(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                match value {
                    Value::Null => visitor.visit_none(),
                    _ => Ok(visitor.visit_some(value)?)
                }
            }
            FieldSelector::Object(field_name, args, _, inner_selector) => {
                let link = self
                    .data
                    .read_link(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                match link {
                    Link::Null => visitor.visit_none(),
                    Link::Single(key) => {
                        let deserializer = ObjectDeserializer::new(
                            self.data,
                            inner_selector,
                            &key,
                            self.guard,
                            self.dependencies
                        );
                        visitor.visit_some(deserializer)
                    }
                    Link::List(keys) => {
                        let deserializer = SeqDeserializer {
                            data: self.data,
                            guard: self.guard,
                            selection: inner_selector,
                            dependencies: self.dependencies,
                            keys: keys.into_iter()
                        };
                        visitor.visit_some(deserializer)
                    }
                }
            }
            FieldSelector::Union(field_name, args, inner_selector) => {
                let link = self
                    .data
                    .read_link(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                match link {
                    Link::Null => visitor.visit_none(),
                    Link::Single(key) => {
                        let typename = self
                            .data
                            .read_record(&key, (&TYPENAME).into(), self.guard)
                            .ok_or_else(|| SerializerError::custom("typename missing"))?;
                        let typename = typename
                            .as_str()
                            .ok_or_else(|| SerializerError::custom("typename not a string"))?;
                        let selection = inner_selector(typename);
                        let deserializer = ObjectDeserializer::new(
                            self.data,
                            &selection,
                            &key,
                            self.guard,
                            self.dependencies
                        );
                        visitor.visit_some(deserializer)
                    }
                    Link::List(keys) => {
                        let deserializer = UnionSeqDeserializer {
                            data: self.data,
                            guard: self.guard,
                            selection: &**inner_selector,
                            keys: keys.into_iter(),
                            dependencies: self.dependencies
                        };
                        visitor.visit_some(deserializer)
                    }
                }
            }
        }
    }

    #[inline]
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V
    ) -> Result<V::Value, Self::Error> {
        match self.selector {
            FieldSelector::Scalar(field_name, args) => {
                let value = self
                    .data
                    .read_record(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                Ok(value.deserialize_enum(name, variants, visitor)?)
            }
            FieldSelector::Union(field_name, args, inner_selection) => {
                let link = self
                    .data
                    .read_link(self.entity_key, field_key(*field_name, args), self.guard)
                    .ok_or_else(|| SerializerError::missing())?;
                match link {
                    Link::Single(key) => {
                        let typename = self.data.read_record(&key, (&TYPENAME).into(), self.guard)
                            .ok_or_else(|| SerializerError::custom("missing typename"))?;
                        let typename = typename.as_str().ok_or_else(|| SerializerError::custom("typename not a string"))?;
                        let selection = inner_selection(typename);
                        let value = ObjectDeserializer::new(
                            self.data,
                            &selection,
                            &key,
                            self.guard,
                            self.dependencies
                        );
                        let deserializer = UnionDeserializer {
                            variant: typename,
                            value
                        };

                        visitor.visit_enum(deserializer)
                    },
                    _ => unreachable!("Arrays or Null can't be deserialized as enums, should be deserialized as Seq/Option")
                }
            }
            _ => unreachable!("Enums are always represented by unions or scalars")
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct UnionSeqDeserializer<'a, 'de> {
    data: &'de InMemoryData,
    guard: &'de Guard,
    keys: <&'a [String] as IntoIterator>::IntoIter,
    selection: &'a dyn Fn(&str) -> Vec<FieldSelector>,
    dependencies: *mut Vec<String>
}

impl<'a, 'de> SeqAccess<'de> for UnionSeqDeserializer<'a, 'de> {
    type Error = SerializerError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.keys.next() {
            Some(key) => {
                let typename = self
                    .data
                    .read_record(key, (&TYPENAME).into(), self.guard)
                    .ok_or_else(|| SerializerError::custom("missing typename"))?;
                let typename = typename
                    .as_str()
                    .ok_or_else(|| SerializerError::custom("typename isn't a string"))?;
                let selection = (self.selection)(typename);
                let deserializer = ObjectDeserializer::new(
                    self.data,
                    &selection,
                    key,
                    self.guard,
                    self.dependencies
                );
                if !self.dependencies.is_null() {
                    // SAFETY: Only one child can exist at once and ObjectDeserializer only writes
                    // to this when there isn't one, so multiple writers are impossible.
                    // Note: This can also be done with `Option<&mut Vec<String>>`, however
                    // profiling shows this adds about 0.5% overhead for what is basically just
                    // a typecheck workaround
                    unsafe { &mut *self.dependencies }.push(key.to_owned());
                }
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.keys.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None
        }
    }
}

impl<'a, 'de> Deserializer<'de> for UnionSeqDeserializer<'a, 'de> {
    type Error = SerializerError;

    #[inline]
    fn deserialize_any<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(&mut self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct SeqDeserializer<'a, 'de> {
    data: &'de InMemoryData,
    guard: &'de Guard,
    keys: <&'a [String] as IntoIterator>::IntoIter,
    selection: &'a [FieldSelector],
    dependencies: *mut Vec<String>
}

impl<'a, 'de> SeqAccess<'de> for SeqDeserializer<'a, 'de> {
    type Error = SerializerError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.keys.next() {
            Some(key) => {
                let deserializer = ObjectDeserializer::new(
                    self.data,
                    &self.selection,
                    key,
                    self.guard,
                    self.dependencies
                );
                if !self.dependencies.is_null() {
                    // SAFETY: Only one child can exist at once and ObjectDeserializer only writes
                    // to this when there isn't one, so multiple writers are impossible.
                    // Note: This can also be done with `Option<&mut Vec<String>>`, however
                    // profiling shows this adds about 0.5% overhead for what is basically just
                    // a typecheck workaround
                    unsafe { &mut *self.dependencies }.push(key.to_owned());
                }
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.keys.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None
        }
    }
}

impl<'a, 'de> Deserializer<'de> for SeqDeserializer<'a, 'de> {
    type Error = SerializerError;

    #[inline]
    fn deserialize_any<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(&mut self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

pub(crate) struct ObjectDeserializer<'a, 'de> {
    data: &'de InMemoryData,
    guard: &'de Guard,
    selection: <&'a [FieldSelector] as IntoIterator>::IntoIter,
    entity_key: &'a str,
    dependencies: *mut Vec<String>,
    value: Option<SelectorDeserializer<'a, 'de>>
}

impl<'a, 'de> ObjectDeserializer<'a, 'de> {
    pub(crate) fn new(
        data: &'de InMemoryData,
        selection: &'a [FieldSelector],
        entity_key: &'a str,
        guard: &'de Guard,
        dependencies: *mut Vec<String>
    ) -> Self {
        Self {
            data,
            selection: selection.into_iter(),
            entity_key,
            guard,
            dependencies,
            value: None
        }
    }
}

impl<'a, 'de> MapAccess<'de> for ObjectDeserializer<'a, 'de> {
    type Error = SerializerError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.selection.next() {
            Some(value) => {
                let key = selector_field_name(&value);
                let value = SelectorDeserializer {
                    data: self.data,
                    guard: self.guard,
                    entity_key: self.entity_key,
                    selector: value,
                    // SAFETY: Only one child can exist at once and ObjectDeserializer only writes
                    // to this when there isn't one, so multiple writers are impossible.
                    // Note: This can also be done with `Option<&mut Vec<String>>`, however
                    // profiling shows this adds about 0.5% overhead for what is basically just
                    // a typecheck workaround
                    dependencies: unsafe { &mut *self.dependencies }
                };
                self.value = Some(value);
                seed.deserialize(key.into_deserializer()).map(Some)
            }
            None => Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V
    ) -> Result<V::Value, Self::Error> {
        match self.value.take() {
            Some(value) => {
                let res = seed.deserialize(value)?;
                Ok(res)
            }
            None => Err(serde::de::Error::custom("value is missing"))
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.selection.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None
        }
    }
}

impl<'a, 'de> Deserializer<'de> for ObjectDeserializer<'a, 'de> {
    type Error = SerializerError;

    #[inline]
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(self)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct UnionDeserializer<'a, 'de> {
    variant: &'a str,
    value: ObjectDeserializer<'a, 'de>
}

impl<'a, 'de> EnumAccess<'de> for UnionDeserializer<'a, 'de> {
    type Error = SerializerError;
    type Variant = VariantDeserializer<'a, 'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant = self.variant.into_deserializer();
        let visitor = VariantDeserializer { value: self.value };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

struct VariantDeserializer<'a, 'de> {
    value: ObjectDeserializer<'a, 'de>
}

impl<'a, 'de> VariantAccess<'de> for VariantDeserializer<'a, 'de> {
    type Error = SerializerError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        unimplemented!("Unions can't have unit variants")
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V>(
        self,
        _len: usize,
        _visitor: V
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        unimplemented!("Unions can't have tuple variants")
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V
    ) -> Result<<V as Visitor<'de>>::Value, Self::Error>
    where
        V: Visitor<'de>
    {
        unimplemented!("Unions can't have struct variants")
    }
}

fn visit_object<'de, V: Visitor<'de>>(
    data: &'de InMemoryData,
    entity_key: &str,
    selection: &[FieldSelector],
    visitor: V,
    guard: &'de Guard,
    dependencies: *mut Vec<String>
) -> Result<V::Value, SerializerError> {
    let mut deserializer =
        ObjectDeserializer::new(data, selection, entity_key, guard, dependencies);
    if !dependencies.is_null() {
        // SAFETY: Only one child can exist at once and ObjectDeserializer only writes
        // to this when there isn't one, so multiple writers are impossible.
        // Note: This can also be done with `Option<&mut Vec<String>>`, however
        // profiling shows this adds about 0.5% overhead for what is basically just
        // a typecheck workaround
        unsafe { &mut *dependencies }.push(entity_key.to_owned());
    }
    visitor.visit_map(&mut deserializer)
}

fn visit_array<'de, V: Visitor<'de>>(
    data: &'de InMemoryData,
    entity_keys: &[String],
    selection: &[FieldSelector],
    visitor: V,
    guard: &'de Guard,
    dependencies: *mut Vec<String>
) -> Result<V::Value, SerializerError> {
    let mut deserializer = SeqDeserializer {
        data,
        keys: entity_keys.into_iter(),
        selection,
        guard,
        dependencies
    };
    visitor.visit_seq(&mut deserializer)
}

fn visit_union_array<'de, V: Visitor<'de>>(
    data: &'de InMemoryData,
    entity_keys: &[String],
    selection: &dyn Fn(&str) -> Vec<FieldSelector>,
    visitor: V,
    guard: &'de Guard,
    dependencies: *mut Vec<String>
) -> Result<V::Value, SerializerError> {
    let mut deserializer = UnionSeqDeserializer {
        data,
        keys: entity_keys.into_iter(),
        selection,
        guard,
        dependencies
    };
    visitor.visit_seq(&mut deserializer)
}
