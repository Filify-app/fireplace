use std::fmt::Display;

use firestore_grpc::v1::Document;
use serde::{
    ser::{self, SerializeMap},
    Serialize,
};

pub struct FirestoreDocument(Document);

impl FirestoreDocument {
    pub(crate) fn new(doc: Document) -> Self {
        Self(doc)
    }
}

#[derive(Debug)]
pub struct SerializeError(anyhow::Error);

impl std::error::Error for SerializeError {}

impl Display for SerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to serialize document: {}", self.0)
    }
}

impl ser::Error for SerializeError {
    fn custom<T: Display>(msg: T) -> Self {
        SerializeError(anyhow::anyhow!(msg.to_string()))
    }
}

impl Serialize for FirestoreDocument {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.fields.len()))?;

        for (key, value) in &self.0.fields {
            map.serialize_entry(key, &FirestoreValue(value))?;
        }

        map.end()
    }
}

pub struct FirestoreValue<'a>(&'a firestore_grpc::v1::Value);

impl<'a> Serialize for FirestoreValue<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use firestore_grpc::v1::value::ValueType::*;

        match &self.0.value_type {
            None => serializer.serialize_unit(),
            Some(value_type) => match value_type {
                NullValue(_) => serializer.serialize_unit(),
                BooleanValue(b) => serializer.serialize_bool(*b),
                IntegerValue(i) => serializer.serialize_i64(*i),
                DoubleValue(f) => serializer.serialize_f64(*f),
                StringValue(s) => serializer.serialize_str(s),
                // TODO: remaining variants
                _ => Err(ser::Error::custom("unsupported value type")),
            },
        }
    }
}
