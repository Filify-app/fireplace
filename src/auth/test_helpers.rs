use std::env;

use crate::ServiceAccount;

use super::{FirebaseAuthClient, FirebaseAuthClientOptions};

/// The client the tests and doc-tests of this crate use.
///
/// With `FIREBASE_AUTH_EMULATOR_HOST` set (as `firebase emulators:exec` sets
/// it) the client talks to that Auth emulator and needs no credentials.
/// Otherwise it talks to the real project the `FIREBASE_*` variables name.
pub fn initialise() -> Result<FirebaseAuthClient, anyhow::Error> {
    if let Ok(emulator_host) = env::var("FIREBASE_AUTH_EMULATOR_HOST") {
        let options = FirebaseAuthClientOptions::default().emulator_host(emulator_host);
        let auth_client = FirebaseAuthClient::with_options(
            crate::emulator::service_account_for_tests(),
            options,
        )?;

        return Ok(auth_client);
    }

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
