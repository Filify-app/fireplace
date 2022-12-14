mod deserialize;
mod serialize;

pub(crate) use deserialize::*;
pub(crate) use serialize::*;

use std::fmt;

use firestore_grpc::v1::value::ValueType;
use serde::{de, ser};

#[derive(Debug)]
pub enum Error {
    /// Any custom error message.
    Message(String),
    /// There were no items left to process.
    Eof,
    /// This error should never surface if the Firebase API docs are correct:
    /// ["Must have a value set."](https://firebase.google.com/docs/firestore/reference/rpc/google.firestore.v1#google.firestore.v1.Value)
    MissingValueType,
    InvalidKey(ValueType),
    InvalidDocument,
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(msg) => formatter.write_str(msg),
            Self::Eof => formatter.write_str("end of content"),
            Self::MissingValueType => formatter.write_str("missing value type"),
            Self::InvalidKey(item) => write!(formatter, "invalid key type: {:?}", item),
            Self::InvalidDocument => {
                formatter.write_str("invalid document; must be a map-like type")
            }
        }
    }
}

impl std::error::Error for Error {}
