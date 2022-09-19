mod deserialize;
mod serialize;

use std::fmt;

pub use deserialize::*;
use serde::{de, ser};
pub use serialize::*;

#[derive(Debug)]
pub enum Error {
    Message(String),
    Eof,
    // TODO: add reference to firestore docs that say this should not be possible
    MissingValueType,
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
        }
    }
}

impl std::error::Error for Error {}
