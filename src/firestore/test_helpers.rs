use std::env;

use serde::Deserialize;

use crate::{
    ServiceAccount,
    firestore::{client::FirestoreClient, collection},
};

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

#[derive(Deserialize, Debug, PartialEq)]
pub struct Landmark {
    pub name: String,
    pub r#type: String,
}

pub async fn setup_landmarks_example(client: &mut FirestoreClient) -> Result<(), anyhow::Error> {
    client
        .set_document(
            &collection("cities")
                .doc("SF")
                .collection("landmarks")
                .doc("golden-gate"),
            &serde_json::json!({ "name": "Golden Gate Bridge", "type": "bridge" }),
        )
        .await?;

    client
        .set_document(
            &collection("cities")
                .doc("SF")
                .collection("landmarks")
                .doc("legion-honor"),
            &serde_json::json!({ "name": "Legion of Honor", "type": "museum" }),
        )
        .await?;

    client
        .set_document(
            &collection("cities")
                .doc("TOK")
                .collection("landmarks")
                .doc("national-science-museum"),
            &serde_json::json!({ "name": "National Museum of Nature and Science", "type": "museum" }),
        )
        .await?;

    Ok(())
}
