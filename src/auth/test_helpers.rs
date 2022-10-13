use std::env;

use crate::token::ServiceAccount;

use super::FirebaseAuthClient;

pub fn initialise() -> Result<FirebaseAuthClient, anyhow::Error> {
    let service_account = ServiceAccount {
        project_id: env::var("FIREBASE_PROJECT_ID")?,
        client_id: env::var("FIREBASE_CLIENT_ID")?,
        client_email: env::var("FIREBASE_CLIENT_EMAIL")?,
        private_key_id: env::var("FIREBASE_PRIVATE_KEY_ID")?,
        private_key: env::var("FIREBASE_PRIVATE_KEY")?.replace(r"\n", "\n"),
    };

    let auth_client = FirebaseAuthClient::new(service_account)?;

    Ok(auth_client)
}
