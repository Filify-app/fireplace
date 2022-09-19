use std::collections::HashMap;

use firestore_grpc::v1::{value::ValueType, ArrayValue, Document, MapValue, Value};
use prost_types::Timestamp;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
    Serialize, Serializer,
};

use super::Error;

pub(crate) fn serialize_to_document<T: Serialize>(
    value: &T,
    name: String,
    create_time: Option<Timestamp>,
    update_time: Option<Timestamp>,
) -> Result<Document, Error> {
    let value_type = serialize(value)?;

    match value_type {
        ValueType::MapValue(map_value) => Ok(Document {
            name,
            create_time,
            update_time,
            fields: map_value.fields,
        }),
        _ => Err(Error::InvalidDocument),
    }
}

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
    type SerializeTuple = TupleSerializer;
    type SerializeTupleStruct = TupleStructSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
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

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let mut inner = HashMap::new();
        inner.insert(
            variant.to_string(),
            Value {
                value_type: Some(value.serialize(self)?),
            },
        );
        Ok(ValueType::MapValue(MapValue { fields: inner }))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ArraySerializer::new(len))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer::new(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleStructSerializer::new(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer::new(variant, len))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new(len))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer::new(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer::new(variant, len))
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
    fn new(size: usize) -> Self {
        Self {
            fields: HashMap::with_capacity(size),
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
    fn new(name: &'static str, size: usize) -> Self {
        Self {
            fields: HashMap::with_capacity(size),
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

struct TupleVariantSerializer {
    values: Vec<Value>,
    name: &'static str,
}

impl TupleVariantSerializer {
    fn new(name: &'static str, len: usize) -> Self {
        Self {
            values: Vec::with_capacity(len),
            name,
        }
    }
}

impl SerializeTupleVariant for TupleVariantSerializer {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let value_type = serialize(value)?;
        self.values.push(Value {
            value_type: Some(value_type),
        });
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let inner = ValueType::ArrayValue(ArrayValue {
            values: self.values,
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

struct TupleStructSerializer {
    values: Vec<Value>,
}

impl TupleStructSerializer {
    fn new(len: usize) -> Self {
        Self {
            values: Vec::with_capacity(len),
        }
    }
}

impl SerializeTupleStruct for TupleStructSerializer {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
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

struct TupleSerializer {
    values: Vec<Value>,
}

impl TupleSerializer {
    fn new(len: usize) -> Self {
        Self {
            values: Vec::with_capacity(len),
        }
    }
}

impl SerializeTuple for TupleSerializer {
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use firestore_grpc::v1::{value::ValueType, ArrayValue, Document, MapValue, Value};
    use serde::Serialize;

    use crate::firestore::serde::serialize_to_document;

    const DOC_NAME: &str = "projects/project-id/databases/(default)/documents/people/luke";

    #[test]
    fn serialize_struct() {
        #[derive(Serialize)]
        struct TestStruct {
            name: String,
            price: i32,
        }

        let value = TestStruct {
            name: "Pep med drez".to_string(),
            price: 65,
        };
        let doc = serialize_to_document(&value, DOC_NAME.to_string(), None, None).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::from(DOC_NAME),
                fields: HashMap::from_iter(vec![
                    (
                        String::from("price"),
                        Value {
                            value_type: Some(ValueType::IntegerValue(65)),
                        },
                    ),
                    (
                        String::from("name"),
                        Value {
                            value_type: Some(ValueType::StringValue(String::from("Pep med drez"))),
                        },
                    ),
                ]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_struct_variant() {
        #[derive(Serialize)]
        #[serde(rename_all = "lowercase")]
        enum TestStructVariant {
            Pepperoni { price: i32 },
            Hawaii { pineapple: bool },
        }

        #[derive(Serialize)]
        struct TestStruct {
            pizza1: TestStructVariant,
            pizza2: TestStructVariant,
        }

        let value = TestStruct {
            pizza1: TestStructVariant::Pepperoni { price: 65 },
            pizza2: TestStructVariant::Hawaii { pineapple: true },
        };
        let doc = serialize_to_document(&value, DOC_NAME.to_string(), None, None).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::from(DOC_NAME),
                fields: HashMap::from_iter(vec![
                    (
                        String::from("pizza1"),
                        Value {
                            value_type: Some(ValueType::MapValue(MapValue {
                                fields: HashMap::from_iter(vec![(
                                    String::from("pepperoni"),
                                    Value {
                                        value_type: Some(ValueType::MapValue(MapValue {
                                            fields: HashMap::from_iter(vec![(
                                                String::from("price"),
                                                Value {
                                                    value_type: Some(ValueType::IntegerValue(65)),
                                                },
                                            )]),
                                        })),
                                    },
                                ),]),
                            }))
                        },
                    ),
                    (
                        String::from("pizza2"),
                        Value {
                            value_type: Some(ValueType::MapValue(MapValue {
                                fields: HashMap::from_iter(vec![(
                                    String::from("hawaii"),
                                    Value {
                                        value_type: Some(ValueType::MapValue(MapValue {
                                            fields: HashMap::from_iter(vec![(
                                                String::from("pineapple"),
                                                Value {
                                                    value_type: Some(ValueType::BooleanValue(true)),
                                                },
                                            )]),
                                        })),
                                    },
                                ),]),
                            }))
                        },
                    )
                ]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_map() {
        let value: HashMap<&str, i32> = HashMap::from_iter([("Pep med drez", 65)]);
        let doc = serialize_to_document(&value, DOC_NAME.to_string(), None, None).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::from(DOC_NAME),
                fields: HashMap::from_iter(vec![(
                    String::from("Pep med drez"),
                    Value {
                        value_type: Some(ValueType::IntegerValue(65)),
                    },
                ),]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_tuple_variant() {
        #[derive(Serialize)]
        #[serde(rename_all = "lowercase")]
        enum TestTupleVariant {
            Pepperoni(i32, &'static str),
        }

        #[derive(Serialize)]
        struct TestStruct {
            pizza: TestTupleVariant,
        }

        let value = TestStruct {
            pizza: TestTupleVariant::Pepperoni(65, "Pep med drez"),
        };
        let doc = serialize_to_document(&value, DOC_NAME.to_string(), None, None).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::from(DOC_NAME),
                fields: HashMap::from_iter(vec![(
                    String::from("pizza"),
                    Value {
                        value_type: Some(ValueType::MapValue(MapValue {
                            fields: HashMap::from_iter(vec![(
                                String::from("pepperoni"),
                                Value {
                                    value_type: Some(ValueType::ArrayValue(ArrayValue {
                                        values: vec![
                                            Value {
                                                value_type: Some(ValueType::IntegerValue(65)),
                                            },
                                            Value {
                                                value_type: Some(ValueType::StringValue(
                                                    String::from("Pep med drez")
                                                )),
                                            }
                                        ],
                                    }))
                                },
                            ),]),
                        }))
                    },
                )]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_tuple_struct() {
        #[derive(Serialize)]
        struct TestTupleStruct(&'static str, i32);

        #[derive(Serialize)]
        struct TestStruct {
            pizza: TestTupleStruct,
        }

        let value = TestStruct {
            pizza: TestTupleStruct("Pep med drez", 65),
        };
        let doc = serialize_to_document(&value, DOC_NAME.to_string(), None, None).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::from(DOC_NAME),
                fields: HashMap::from_iter(vec![(
                    String::from("pizza"),
                    Value {
                        value_type: Some(ValueType::ArrayValue(ArrayValue {
                            values: vec![
                                Value {
                                    value_type: Some(ValueType::StringValue(String::from(
                                        "Pep med drez"
                                    ))),
                                },
                                Value {
                                    value_type: Some(ValueType::IntegerValue(65)),
                                },
                            ],
                        }))
                    },
                )]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_tuple() {
        #[derive(Serialize)]
        struct TestStruct {
            pizza: (&'static str, i32),
        }

        let value = TestStruct {
            pizza: ("Pep med drez", 65),
        };
        let doc = serialize_to_document(&value, DOC_NAME.to_string(), None, None).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::from(DOC_NAME),
                fields: HashMap::from_iter(vec![(
                    String::from("pizza"),
                    Value {
                        value_type: Some(ValueType::ArrayValue(ArrayValue {
                            values: vec![
                                Value {
                                    value_type: Some(ValueType::StringValue(String::from(
                                        "Pep med drez"
                                    ))),
                                },
                                Value {
                                    value_type: Some(ValueType::IntegerValue(65)),
                                },
                            ],
                        }))
                    },
                )]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_seq() {
        #[derive(Serialize)]
        struct TestStruct {
            toppings: Vec<&'static str>,
        }

        let value = TestStruct {
            toppings: vec!["pep", "drez"],
        };
        let doc = serialize_to_document(&value, DOC_NAME.to_string(), None, None).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::from(DOC_NAME),
                fields: HashMap::from_iter(vec![(
                    String::from("toppings"),
                    Value {
                        value_type: Some(ValueType::ArrayValue(ArrayValue {
                            values: vec![
                                Value {
                                    value_type: Some(ValueType::StringValue(String::from("pep"))),
                                },
                                Value {
                                    value_type: Some(ValueType::StringValue(String::from("drez"))),
                                }
                            ],
                        }))
                    },
                )]),
                create_time: None,
                update_time: None,
            }
        );
    }
}
