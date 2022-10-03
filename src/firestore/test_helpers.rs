use std::env;

use crate::{
    firestore::client::FirestoreClient,
    token::{FirebaseTokenProvider, ServiceAccount},
};

pub async fn initialise() -> Result<FirestoreClient, anyhow::Error> {
    let service_account = ServiceAccount {
        project_id: env::var("FIREBASE_PROJECT_ID")?,
        client_id: env::var("FIREBASE_CLIENT_ID")?,
        client_email: env::var("FIREBASE_CLIENT_EMAIL")?,
        private_key_id: env::var("FIREBASE_PRIVATE_KEY_ID")?,
        private_key: env::var("FIREBASE_PRIVATE_KEY")?.replace(r"\n", "\n"),
    };

    let project_id = service_account.project_id.clone();
    let token_provider = FirebaseTokenProvider::new(service_account);

    let client = FirestoreClient::initialise(&project_id, token_provider)
        .await
        .unwrap();

    Ok(client)
}
