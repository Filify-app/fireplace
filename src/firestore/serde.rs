use std::collections::hash_map;
use std::fmt::{self, Display};
use std::vec;

use firestore_grpc::v1::value::ValueType;
use serde::de::{DeserializeSeed, MapAccess, SeqAccess};
use serde::Deserialize;
use serde::{
    de::{self, Visitor},
    ser,
};

pub fn deserialize_firestore_document<'de, T: Deserialize<'de>>(
    doc: firestore_grpc::v1::Document,
) -> Result<T, Error> {
    // The Document struct is essentially just a map but with extra fields like
    // create/update timestamps. Deserializing it becomes easy if we just turn
    // it into an explicit map.
    let value = ValueType::MapValue(firestore_grpc::v1::MapValue { fields: doc.fields });
    let deserializer = FirestoreValueDeserializer { value };
    let result = T::deserialize(deserializer)?;
    Ok(result)
}

#[derive(Debug)]
pub enum Error {
    Message(String),
    Eof,
    // TODO: add reference to firestore docs that say this should not be possible
    MissingValueType,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(msg) => formatter.write_str(msg),
            Self::Eof => formatter.write_str("end of content"),
            Self::MissingValueType => formatter.write_str("missing value type"),
        }
    }
}

impl std::error::Error for Error {}

struct FirestoreValueDeserializer {
    value: ValueType,
}

impl<'de> de::Deserializer<'de> for FirestoreValueDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        use ValueType::*;

        match self.value {
            NullValue(_) => visitor.visit_unit(),
            BooleanValue(b) => visitor.visit_bool(b),
            IntegerValue(i) => visitor.visit_i64(i),
            DoubleValue(f) => visitor.visit_f64(f),
            StringValue(s) => visitor.visit_str(&s),
            MapValue(m) => visitor.visit_map(MapDeserializer::new(m)),
            ArrayValue(a) => visitor.visit_seq(ArrayDeserializer::new(a)),
            // TODO: remaining variants
            TimestampValue(_) => todo!(),
            ReferenceValue(_) => todo!(),
            BytesValue(_) => todo!(),
            GeoPointValue(_) => todo!(),
        }
    }

    fn is_human_readable(&self) -> bool {
        true
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct MapDeserializer {
    fields: hash_map::IntoIter<String, firestore_grpc::v1::Value>,
    len: usize,
    value: Option<ValueType>,
}

impl MapDeserializer {
    fn new(map: firestore_grpc::v1::MapValue) -> Self {
        Self {
            len: map.fields.len(),
            fields: map.fields.into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.fields.next() {
            Some((key, value_wrapper)) => {
                let value = match value_wrapper.value_type {
                    Some(vt) => vt,
                    None => return Err(Error::MissingValueType),
                };

                self.len -= 1;
                self.value = Some(value);

                let de = FirestoreValueDeserializer {
                    value: ValueType::StringValue(key),
                };

                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self.value.take().ok_or(Error::Eof)?;
        let de = FirestoreValueDeserializer { value };
        seed.deserialize(de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct ArrayDeserializer {
    iter: vec::IntoIter<firestore_grpc::v1::Value>,
    len: usize,
}

impl ArrayDeserializer {
    fn new(arr: firestore_grpc::v1::ArrayValue) -> Self {
        Self {
            len: arr.values.len(),
            iter: arr.values.into_iter(),
        }
    }
}

impl<'de> SeqAccess<'de> for ArrayDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            None => Ok(None),
            Some(value_wrapper) => {
                let value = match value_wrapper.value_type {
                    Some(vt) => vt,
                    None => return Err(Error::MissingValueType),
                };

                self.len -= 1;

                let de = FirestoreValueDeserializer { value };
                seed.deserialize(de).map(Some)
            }
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}
