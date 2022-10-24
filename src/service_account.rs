use std::{fs::File, path::Path};

use anyhow::Context;
use serde::Deserialize;

use crate::error::FirebaseError;

/// Service account information contained within the service account JSON file
/// that you can download from Firebase.
///
/// `Serialize`, `Display`, and `Debug` are intentionally not implemented to
/// avoid accidentally leaking credentials.
#[derive(Deserialize, Clone)]
pub struct ServiceAccount {
    pub project_id: String,
    pub private_key: String,
    pub private_key_id: String,
    pub client_email: String,
    pub client_id: String,
}

impl ServiceAccount {
    /// Creates a new `ServiceAccount` instance from a service account JSON
    /// file. You can download such a file from Firebase.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, FirebaseError> {
        let file_reader = File::open(path).context("Failed to read service account JSON file")?;
        let service_account = serde_json::from_reader(file_reader)
            .context("Could not extract service account details from file")?;

        Ok(service_account)
    }
}
