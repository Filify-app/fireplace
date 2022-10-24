use std::env;

use crate::{firestore::client::FirestoreClient, ServiceAccount};

use super::client::FirestoreClientOptions;

pub async fn initialise() -> Result<FirestoreClient, anyhow::Error> {
    let service_account = ServiceAccount {
        project_id: env::var("FIREBASE_PROJECT_ID")?,
        client_id: env::var("FIREBASE_CLIENT_ID")?,
        client_email: env::var("FIREBASE_CLIENT_EMAIL")?,
        private_key_id: env::var("FIREBASE_PRIVATE_KEY_ID")?,
        private_key: env::var("FIREBASE_PRIVATE_KEY")?.replace(r"\n", "\n"),
    };

    let client_options = FirestoreClientOptions::default();
    let client = FirestoreClient::initialise(service_account, client_options)
        .await
        .unwrap();

    Ok(client)
}
