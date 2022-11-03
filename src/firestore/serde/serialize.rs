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

use crate::firestore::reference::{CollectionReference, DocumentReference};

use super::Error;

pub(crate) struct DocumentSerializer {
    root_resource_path: String,
    name: Option<String>,
    create_time: Option<Timestamp>,
    update_time: Option<Timestamp>,
}

impl DocumentSerializer {
    pub fn new(root_resource_path: impl Into<String>) -> Self {
        Self {
            root_resource_path: root_resource_path.into(),
            name: None,
            create_time: None,
            update_time: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn create_time(mut self, timestamp: Timestamp) -> Self {
        self.create_time = Some(timestamp);
        self
    }

    pub fn update_time(mut self, timestamp: Timestamp) -> Self {
        self.update_time = Some(timestamp);
        self
    }

    pub fn serialize<T: Serialize>(self, value: &T) -> Result<Document, Error> {
        let value_type = serialize(value, &self.root_resource_path)?;

        match value_type {
            ValueType::MapValue(map_value) => Ok(Document {
                create_time: self.create_time,
                update_time: self.update_time,
                name: self.name.unwrap_or_default(),
                fields: map_value.fields,
            }),
            _ => Err(Error::InvalidDocument),
        }
    }
}

pub(crate) fn serialize_to_value_type<T: Serialize>(
    value: &T,
    root_resource_path: &str,
) -> Result<ValueType, Error> {
    let value_type = serialize(value, root_resource_path)?;
    Ok(value_type)
}

struct FirestoreValueSerializer<'a> {
    root_resource_path: &'a str,
}

impl<'a> Serializer for FirestoreValueSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    type SerializeSeq = ArraySerializer<'a>;
    type SerializeTuple = TupleSerializer<'a>;
    type SerializeTupleStruct = TupleStructSerializer<'a>;
    type SerializeTupleVariant = TupleVariantSerializer<'a>;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = StructSerializerKind<'a>;
    type SerializeStructVariant = StructVariantSerializer<'a>;

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
        Ok(ArraySerializer::new(len, self.root_resource_path))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(TupleSerializer::new(len, self.root_resource_path))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(TupleStructSerializer::new(len, self.root_resource_path))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(TupleVariantSerializer::new(
            variant,
            len,
            self.root_resource_path,
        ))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer::new(len, self.root_resource_path))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let struct_serializer =
            if name == DocumentReference::type_id() || name == CollectionReference::type_id() {
                StructSerializerKind::ReferenceValue(ReferenceTypeSerializer::new(
                    self.root_resource_path,
                ))
            } else {
                StructSerializerKind::Other(StructSerializer::new(len, self.root_resource_path))
            };

        Ok(struct_serializer)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructVariantSerializer::new(
            variant,
            len,
            self.root_resource_path,
        ))
    }
}

fn serialize<T: ?Sized + Serialize>(
    value: &T,
    root_resource_path: &str,
) -> Result<ValueType, Error> {
    let serializer = FirestoreValueSerializer { root_resource_path };
    value.serialize(serializer)
}

struct ArraySerializer<'a> {
    values: Vec<Value>,
    root_resource_path: &'a str,
}

impl<'a> ArraySerializer<'a> {
    fn new(len: Option<usize>, root_resource_path: &'a str) -> Self {
        Self {
            values: match len {
                Some(l) => Vec::with_capacity(l),
                None => Vec::new(),
            },
            root_resource_path,
        }
    }
}

impl<'a> SerializeSeq for ArraySerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let value_type = serialize(value, self.root_resource_path)?;
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

struct MapSerializer<'a> {
    fields: HashMap<String, Value>,
    next_key: Option<String>,
    root_resource_path: &'a str,
}

impl<'a> MapSerializer<'a> {
    fn new(size: Option<usize>, root_resource_path: &'a str) -> Self {
        Self {
            fields: match size {
                Some(s) => HashMap::with_capacity(s),
                None => HashMap::new(),
            },
            next_key: None,
            root_resource_path,
        }
    }
}

impl<'a> SerializeMap for MapSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.next_key = match serialize(key, self.root_resource_path)? {
            ValueType::StringValue(s) => Some(s),
            other => return Err(Error::InvalidKey(other)),
        };
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self.next_key.take().unwrap_or_default();
        let value_type = serialize(value, self.root_resource_path)?;
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

enum StructSerializerKind<'a> {
    ReferenceValue(ReferenceTypeSerializer<'a>),
    Other(StructSerializer<'a>),
}

impl<'a> SerializeStruct for StructSerializerKind<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        match self {
            StructSerializerKind::ReferenceValue(r) => r.serialize_field(key, value),
            StructSerializerKind::Other(o) => o.serialize_field(key, value),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self {
            StructSerializerKind::ReferenceValue(r) => r.end(),
            StructSerializerKind::Other(o) => o.end(),
        }
    }
}

struct StructSerializer<'a> {
    fields: HashMap<String, Value>,
    root_resource_path: &'a str,
}

impl<'a> StructSerializer<'a> {
    fn new(size: usize, root_resource_path: &'a str) -> Self {
        Self {
            fields: HashMap::with_capacity(size),
            root_resource_path,
        }
    }
}

impl<'a> SerializeStruct for StructSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let value_type = serialize(value, self.root_resource_path)?;
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

struct ReferenceTypeSerializer<'a> {
    relative_path: Option<String>,
    root_resource_path: &'a str,
}

impl<'a> ReferenceTypeSerializer<'a> {
    fn new(root_resource_path: &'a str) -> Self {
        Self {
            relative_path: None,
            root_resource_path,
        }
    }
}

const REF_TYPE_RELATIVE_PATH_KEY: &str = "relative_path";

impl<'a> SerializeStruct for ReferenceTypeSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        match (key, serialize(value, self.root_resource_path)?) {
            (REF_TYPE_RELATIVE_PATH_KEY, ValueType::StringValue(s)) => {
                self.relative_path = Some(s);
                Ok(())
            }
            _ => Err(Error::Message(
                "expected valid relative path for reference".into(),
            )),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.relative_path
            .map(|rel_path| {
                ValueType::ReferenceValue(format!("{}/{}", self.root_resource_path, rel_path))
            })
            .ok_or_else(|| {
                Error::Message(format!(
                    "missing key {} on firestore reference value",
                    REF_TYPE_RELATIVE_PATH_KEY
                ))
            })
    }
}

struct StructVariantSerializer<'a> {
    fields: HashMap<String, Value>,
    name: &'static str,
    root_resource_path: &'a str,
}

impl<'a> StructVariantSerializer<'a> {
    fn new(name: &'static str, size: usize, root_resource_path: &'a str) -> Self {
        Self {
            fields: HashMap::with_capacity(size),
            name,
            root_resource_path,
        }
    }
}

impl<'a> SerializeStructVariant for StructVariantSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        let value_type = serialize(value, self.root_resource_path)?;
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

struct TupleVariantSerializer<'a> {
    values: Vec<Value>,
    name: &'static str,
    root_resource_path: &'a str,
}

impl<'a> TupleVariantSerializer<'a> {
    fn new(name: &'static str, len: usize, root_resource_path: &'a str) -> Self {
        Self {
            values: Vec::with_capacity(len),
            name,
            root_resource_path,
        }
    }
}

impl<'a> SerializeTupleVariant for TupleVariantSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let value_type = serialize(value, self.root_resource_path)?;
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

struct TupleStructSerializer<'a> {
    values: Vec<Value>,
    root_resource_path: &'a str,
}

impl<'a> TupleStructSerializer<'a> {
    fn new(len: usize, root_resource_path: &'a str) -> Self {
        Self {
            values: Vec::with_capacity(len),
            root_resource_path,
        }
    }
}

impl<'a> SerializeTupleStruct for TupleStructSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let value_type = serialize(value, self.root_resource_path)?;
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

struct TupleSerializer<'a> {
    values: Vec<Value>,
    root_resource_path: &'a str,
}

impl<'a> TupleSerializer<'a> {
    fn new(len: usize, root_resource_path: &'a str) -> Self {
        Self {
            values: Vec::with_capacity(len),
            root_resource_path,
        }
    }
}

impl<'a> SerializeTuple for TupleSerializer<'a> {
    type Ok = ValueType;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let value_type = serialize(value, self.root_resource_path)?;
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

    use crate::firestore::{
        collection,
        reference::{CollectionReference, DocumentReference},
        serde::DocumentSerializer,
    };

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
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
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
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
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
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
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
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
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
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
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
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
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
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
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

    #[test]
    fn serialize_option() {
        #[derive(Serialize)]
        struct TestStruct {
            name: Option<&'static str>,
            topping: Option<&'static str>,
        }

        let value = TestStruct {
            name: Some("bread"),
            topping: None,
        };
        let doc = DocumentSerializer::new("").serialize(&value).unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
                fields: HashMap::from_iter(vec![
                    (
                        String::from("name"),
                        Value {
                            value_type: Some(ValueType::StringValue(String::from("bread"))),
                        },
                    ),
                    (
                        String::from("topping"),
                        Value {
                            value_type: Some(ValueType::NullValue(0)),
                        },
                    )
                ]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_document_reference() {
        #[derive(Serialize)]
        struct TestStruct {
            pizza_ref: DocumentReference,
        }

        let value = TestStruct {
            pizza_ref: collection("pizzas").doc("pep"),
        };
        let doc = DocumentSerializer::new("projects/pizzaproject/databases/(default)/documents")
            .serialize(&value)
            .unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
                fields: HashMap::from_iter(vec![(
                    String::from("pizza_ref"),
                    Value {
                        value_type: Some(ValueType::ReferenceValue(String::from(
                            "projects/pizzaproject/databases/(default)/documents/pizzas/pep"
                        )))
                    },
                ),]),
                create_time: None,
                update_time: None,
            }
        );
    }

    #[test]
    fn serialize_collection_reference() {
        #[derive(Serialize)]
        struct TestStruct {
            toppings_ref: CollectionReference,
        }

        let value = TestStruct {
            toppings_ref: collection("pizzas").doc("pep").collection("toppings"),
        };
        let doc = DocumentSerializer::new("projects/pizzaproject/databases/(default)/documents")
            .serialize(&value)
            .unwrap();

        assert_eq!(
            doc,
            Document {
                name: String::new(),
                fields: HashMap::from_iter(vec![(
                    String::from("toppings_ref"),
                    Value {
                        value_type: Some(ValueType::ReferenceValue(String::from(
                            "projects/pizzaproject/databases/(default)/documents/pizzas/pep/toppings"
                        )))
                    },
                ),]),
                create_time: None,
                update_time: None,
            }
        );
    }
}
