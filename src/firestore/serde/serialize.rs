use std::collections::HashMap;

use firestore_grpc::v1::{value::ValueType, ArrayValue, MapValue, Value};
use serde::{
    ser::{SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant},
    Serialize, Serializer,
};

use super::Error;

struct FirestoreValueSerializer;

impl FirestoreValueSerializer {
    fn new() -> Self {
        Self
    }
}

impl Serializer for FirestoreValueSerializer {
    type Ok = ValueType;
    type Error = Error;

    type SerializeSeq = ArraySerializer;
    type SerializeTuple;
    type SerializeTupleStruct;
    type SerializeTupleVariant;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::BooleanValue(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::IntegerValue(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    /// Beware, this might overflow since the value is casted to a 64-bit
    /// signed integer because that's the only integer type supported in
    /// Firestore.
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::IntegerValue(v as i64))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::DoubleValue(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let mut char_str = [0; 4];
        self.serialize_str(v.encode_utf8(&mut char_str))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::StringValue(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::BytesValue(v.to_vec()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::NullValue(0))
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ArraySerializer::new(len))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new(len))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        todo!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}

fn serialize<T: ?Sized + Serialize>(value: &T) -> Result<ValueType, Error> {
    let serializer = FirestoreValueSerializer::new();
    value.serialize(serializer)
}

struct ArraySerializer {
    values: Vec<Value>,
}

impl ArraySerializer {
    fn new(len: Option<usize>) -> Self {
        Self {
            values: match len {
                Some(l) => Vec::with_capacity(l),
                None => Vec::new(),
            },
        }
    }
}

impl SerializeSeq for ArraySerializer {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let value_type = serialize(value)?;
        self.values.push(Value {
            value_type: Some(value_type),
        });
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::ArrayValue(ArrayValue {
            values: self.values,
        }))
    }
}

struct MapSerializer {
    fields: HashMap<String, Value>,
    next_key: Option<String>,
}

impl MapSerializer {
    fn new(size: Option<usize>) -> Self {
        Self {
            fields: match size {
                Some(s) => HashMap::with_capacity(s),
                None => HashMap::new(),
            },
            next_key: None,
        }
    }
}

impl SerializeMap for MapSerializer {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.next_key = match serialize(key)? {
            ValueType::StringValue(s) => Some(s),
            other => return Err(Error::InvalidKey(other)),
        };
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self.next_key.take().unwrap_or_default();
        let value_type = serialize(value)?;
        self.fields.insert(
            key,
            Value {
                value_type: Some(value_type),
            },
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::MapValue(MapValue {
            fields: self.fields,
        }))
    }
}

struct StructSerializer {
    fields: HashMap<String, Value>,
}

impl StructSerializer {
    fn new(size: Option<usize>) -> Self {
        Self {
            fields: match size {
                Some(s) => HashMap::with_capacity(s),
                None => HashMap::new(),
            },
        }
    }
}

impl SerializeStruct for StructSerializer {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let value_type = serialize(value)?;
        self.fields.insert(
            key.to_string(),
            Value {
                value_type: Some(value_type),
            },
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(ValueType::MapValue(MapValue {
            fields: self.fields,
        }))
    }
}

struct StructVariantSerializer {
    fields: HashMap<String, Value>,
    name: &'static str,
}

impl StructVariantSerializer {
    fn new(name: &'static str, size: Option<usize>) -> Self {
        Self {
            fields: match size {
                Some(s) => HashMap::with_capacity(s),
                None => HashMap::new(),
            },
            name,
        }
    }
}

impl SerializeStructVariant for StructVariantSerializer {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let value_type = serialize(value)?;
        self.fields.insert(
            key.to_string(),
            Value {
                value_type: Some(value_type),
            },
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let inner = ValueType::MapValue(MapValue {
            fields: self.fields,
        });

        let mut outer = HashMap::new();
        outer.insert(
            self.name.to_string(),
            Value {
                value_type: Some(inner),
            },
        );

        Ok(ValueType::MapValue(MapValue { fields: outer }))
    }
}
