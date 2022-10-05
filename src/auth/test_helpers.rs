use std::env;

use anyhow::Context;

use super::FirebaseAuthClient;

pub fn initialise() -> Result<FirebaseAuthClient, anyhow::Error> {
    let api_key = env::var("FIREBASE_API_KEY").context("Missing FIREBASE_API_KEY")?;
    let project_id = env::var("FIREBASE_PROJECT_ID").context("Missing FIREBASE_PROJECT_ID")?;
    let auth_client = FirebaseAuthClient::new(project_id, &api_key)?;
    Ok(auth_client)
}
