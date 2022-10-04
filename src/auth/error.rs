use anyhow::anyhow;
use serde::Deserialize;

use crate::error::FirebaseError;

#[derive(Debug, Deserialize)]
pub(crate) struct AuthApiError {
    error: AuthApiErrorInfo,
}

#[derive(Debug, Deserialize)]
pub struct AuthApiErrorInfo {
    pub message: String,
    pub errors: Vec<SpecificAuthApiErrorInfo>,
    pub code: u16,
}

#[derive(Debug, Deserialize)]
pub struct SpecificAuthApiErrorInfo {
    pub domain: String,
    pub message: String,
    pub reason: String,
}

impl From<AuthApiError> for FirebaseError {
    fn from(err: AuthApiError) -> Self {
        match err.error.message.as_ref() {
            "EMAIL_EXISTS" => FirebaseError::EmailAlreadyExists,
            _ => anyhow!("{:?}", err).into(),
        }
    }
}
