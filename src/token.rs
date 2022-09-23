use std::{fs::File, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::error::FirebaseError;

/// Service account information contained within the service account JSON file
/// that you can download from Firebase.
///
/// `Serialize`, `Display`, and `Debug` are intentionally not implemented to
/// avoid accidentally leaking credentials.
#[derive(Deserialize)]
pub struct ServiceAccount {
    project_id: String,
    private_key: String,
    private_key_id: String,
    client_email: String,
    client_id: String,
}

pub struct FirebaseTokenProvider {
    service_account: ServiceAccount,
}

impl FirebaseTokenProvider {
    /// Creates a new `FirebaseAuth` instance from a service account JSON file.
    /// You can download such a file from Firebase.
    pub fn from_service_account_file(
        path: impl AsRef<Path>,
    ) -> Result<FirebaseTokenProvider, FirebaseError> {
        let file_reader = File::open(path).context("Failed to read service account JSON file")?;
        let service_account = serde_json::from_reader(file_reader)
            .context("Could not extract service account details from file")?;

        Ok(FirebaseTokenProvider { service_account })
    }

    pub fn get_token(&self) -> Result<String, FirebaseError> {
        // TODO: Reuse token if it's still valid and regenerate it if it's not
        let token = create_jwt(&self.service_account)?;
        Ok(token)
    }

    pub fn project_id(&self) -> &str {
        &self.service_account.project_id
    }
}

fn create_jwt(service_account: &ServiceAccount) -> Result<String, anyhow::Error> {
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(service_account.private_key_id.clone());

    let issued_at_time = jsonwebtoken::get_current_timestamp();
    let claims = JwtClaims {
        iss: &service_account.client_email,
        sub: &service_account.client_email,
        // TODO: This is something I had to find in some random place. The official aud URL
        // doesn't work. How to fix?
        aud: "https://firestore.googleapis.com/",
        iat: issued_at_time,
        exp: issued_at_time + 3600,
        uid: &service_account.client_id,
    };

    let encoding_key =
        jsonwebtoken::EncodingKey::from_rsa_pem(service_account.private_key.as_ref())?;

    jsonwebtoken::encode(&header, &claims, &encoding_key).context("Failed to create JWT")
}

#[derive(Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
    uid: &'a str,
}
