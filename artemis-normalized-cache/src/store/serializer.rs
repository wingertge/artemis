use crate::store::data::{FieldKey, InMemoryData, Link};
use artemis::codegen::FieldSelector;
use flurry::epoch::Guard;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTupleVariant
    },
    Serialize, Serializer
};
use serde_json::Value;
use std::fmt::Display;
use serde::ser::{SerializeTuple, SerializeTupleStruct};
use crate::Dependencies;

enum Field {
    Value(Value),
    Link(Link)
}

pub struct ObjectSerializer<'a, 'g> {
    data: &'g InMemoryData,
    guard: &'g Guard,
    selection: &'a [FieldSelector],
    selection_iter: <&'a [FieldSelector] as IntoIterator>::IntoIter,
    typename: &'a str,
    entity_key: Option<String>,
    fields: Vec<Field>,
    dependencies: *mut Dependencies,
    optimistic_key: Option<u64>
}

impl<'a, 'g> ObjectSerializer<'a, 'g> {
    pub fn new(
        data: &'g InMemoryData,
        guard: &'g Guard,
        selection: &'a [FieldSelector],
        typename: &'a str,
        entity_key: Option<String>,
        dependencies: *mut Dependencies,
        optimistic_key: Option<u64>
    ) -> Self {
        let len = selection.len();
        ObjectSerializer {
            data,
            guard,
            selection,
            selection_iter: selection.into_iter(),
            typename,
            entity_key,
            fields: Vec::with_capacity(len),
            dependencies,
            optimistic_key
        }
    }
}

pub struct Unimpl;
impl SerializeTuple for Unimpl {
    type Ok = Link;
    type Error = serde_json::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where
        T: Serialize {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}
impl SerializeTupleStruct for Unimpl {
    type Ok = Link;
    type Error = serde_json::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where
        T: Serialize {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}
impl SerializeTupleVariant for Unimpl {
    type Ok = Link;
    type Error = serde_json::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}
impl SerializeStructVariant for Unimpl {
    type Ok = Link;
    type Error = serde_json::Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T
    ) -> Result<(), Self::Error>
    where
        T: Serialize
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}
impl SerializeMap for Unimpl {
    type Ok = Link;
    type Error = serde_json::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> where
        T: Serialize {
        unimplemented!()
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where
        T: Serialize {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

impl<'a, 'g> Serializer for ObjectSerializer<'a, 'g> {
    type Ok = Link;
    type Error = serde_json::Error;

    type SerializeSeq = SerializeVec<'a, 'g>;
    type SerializeTuple = Unimpl;
    type SerializeTupleStruct = Unimpl;
    type SerializeTupleVariant = Unimpl;
    type SerializeMap = Unimpl;
    type SerializeStruct = Self;
    type SerializeStructVariant = Unimpl;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Link::Null)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize
    {
        let link = value.serialize(self)?;
        Ok(link)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str
    ) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize
    {
        unimplemented!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize
    {
        unimplemented!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeVec {
            data: self.data,
            guard: self.guard,
            selection: self.selection,
            typename: self.typename,
            entity_keys: Vec::with_capacity(len.unwrap_or(0)),
            dependencies: self.dependencies,
            optimistic_key: self.optimistic_key
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct(
        mut self,
        _name: &'static str,
        _len: usize
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!()
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Display
    {
        unimplemented!()
    }
}

impl<'a, 'g> SerializeStruct for ObjectSerializer<'a, 'g> {
    type Ok = Link;
    type Error = serde_json::Error;

    fn serialize_field<V: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &V
    ) -> Result<(), Self::Error> {
        let key = key as *const _ as *const str;
        let key = unsafe { &*key };

        if key == "id" || key == "_id" {
            self.selection_iter.next();
            let value = value.serialize(serde_json::value::Serializer)?;
            self.entity_key = Some(entity_key(&self.typename, value.as_str().unwrap()));
            self.fields.push(Field::Value(value));
        } else {
            match self.selection_iter.next().unwrap() {
                FieldSelector::Scalar(_, _) => {
                    let value = value.serialize(serde_json::value::Serializer)?;
                    self.fields.push(Field::Value(value));
                }
                FieldSelector::Object(_, _, typename, inner_selection) => {
                    let entity_key = value.serialize(ObjectSerializer::new(
                        self.data,
                        self.guard,
                        &inner_selection,
                        typename,
                        None,
                        self.dependencies,
                        self.optimistic_key
                    ))?;
                    self.fields.push(Field::Link(entity_key));
                }
                FieldSelector::Union(_, _, inner_selection) => todo!("Unions")
            }
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let entity_key = self.entity_key.expect("Entity key not found");
        if &entity_key != "Query" {
            unsafe { &mut *self.dependencies }.insert(entity_key.clone());
        }

        let mut fields = self.fields.into_iter();
        for selector in self.selection {
            let value = fields.next().unwrap();
            match selector {
                FieldSelector::Scalar(field_name, args) => {
                    let value = match value {
                        Field::Value(value) => value,
                        _ => unreachable!()
                    };
                    write_record(
                        &self.data,
                        self.optimistic_key,
                        &entity_key,
                        FieldKey(*field_name, args.to_owned()),
                        Some(value),
                        self.guard
                    );
                }
                FieldSelector::Object(field_name, args, typename, _) => {
                    let value = match value {
                        Field::Link(key) => key,
                        _ => unreachable!()
                    };
                    write_link(
                        &self.data,
                        self.optimistic_key,
                        &entity_key,
                        FieldKey(*field_name, args.to_owned()),
                        Some(value),
                        self.guard
                    );
                }
                FieldSelector::Union(_, _, _) => todo!("Unions")
            }
        }

        Ok(Link::Single(entity_key))
    }
}

fn entity_key(typename: &str, key: &str) -> String {
    let mut s = String::with_capacity(typename.len() + key.len() + 1);
    s.push_str(typename);
    s.push_str(":");
    s.push_str(key);
    s
}

pub struct SerializeVec<'a, 'g> {
    data: &'g InMemoryData,
    guard: &'g Guard,
    selection: &'a [FieldSelector],
    typename: &'a str,
    entity_keys: Vec<String>,
    dependencies: *mut Dependencies,
    optimistic_key: Option<u64>
}

impl<'a, 'g> SerializeSeq for SerializeVec<'a, 'g> {
    type Ok = Link;
    type Error = serde_json::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> where
        T: Serialize {
        let serializer = ObjectSerializer::new(
            self.data,
            self.guard,
            self.selection,
            self.typename,
            None,
            self.dependencies,
            self.optimistic_key
        );
        let link = value.serialize(serializer)?;
        match link {
            Link::Single(s) => self.entity_keys.push(s),
            _ => unreachable!()
        };
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Link::List(self.entity_keys))
    }
}

fn write_record(
    data: &InMemoryData,
    optimistic_key: Option<u64>,
    entity_key: &str,
    field_key: FieldKey,
    value: Option<Value>,
    guard: &Guard
) {
    if let Some(optimistic_key) = optimistic_key {
        data.write_record_optimistic(optimistic_key, entity_key, field_key, value, guard);
    } else {
        data.write_record(entity_key, field_key, value, guard);
    }
}

fn write_link(
    data: &InMemoryData,
    optimistic_key: Option<u64>,
    entity_key: &str,
    field_key: FieldKey,
    value: Option<Link>,
    guard: &Guard
) {
    if let Some(optimistic_key) = optimistic_key {
        data
            .write_link_optimistic(optimistic_key, entity_key, field_key, value, guard);
    } else if let Some(value) = value {
        // Non-optimistic writes only support insertion
        data.write_link(entity_key, field_key, value, guard);
    }
}
