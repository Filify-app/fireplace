use anyhow::anyhow;
use serde::Deserialize;

use crate::error::FirebaseError;

#[derive(Debug, Deserialize)]
pub(crate) struct AuthApiError {
    error: AuthApiErrorInfo,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthApiErrorInfo {
    message: String,
    errors: Vec<SpecificAuthApiErrorInfo>,
    code: u16,
}

#[derive(Debug, Deserialize)]
struct SpecificAuthApiErrorInfo {
    domain: String,
    message: String,
    reason: String,
}

impl From<AuthApiError> for FirebaseError {
    fn from(err: AuthApiError) -> Self {
        match err.error.message.as_ref() {
            "EMAIL_EXISTS" => FirebaseError::EmailAlreadyExists,
            _ => anyhow!("{:?}", err).into(),
        }
    }
}

impl<T> From<AuthApiError> for Result<T, FirebaseError> {
    fn from(err: AuthApiError) -> Self {
        Err(err.into())
    }
}
