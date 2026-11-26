//! What the two clients share when they talk to the [Firebase Local Emulator
//! Suite] instead of Google.
//!
//! Emulator mode is always asked for explicitly, through
//! [`FirestoreClientOptions::emulator_host`] and
//! [`FirebaseAuthClientOptions::emulator_host`]. Unlike the official SDKs,
//! this crate never reads `FIRESTORE_EMULATOR_HOST` or
//! `FIREBASE_AUTH_EMULATOR_HOST` on its own: an environment variable set on a
//! production host must not be able to switch off the signature check on ID
//! tokens.
//!
//! As a second lock, emulator mode is refused for any project whose ID does
//! not start with `demo-`. Firebase reserves that prefix for projects that
//! exist only in the emulators, so no real project can end up accepting the
//! unsigned tokens the Auth emulator issues.
//!
//! [Firebase Local Emulator Suite]: https://firebase.google.com/docs/emulator-suite
//! [`FirestoreClientOptions::emulator_host`]: crate::firestore::client::FirestoreClientOptions::emulator_host
//! [`FirebaseAuthClientOptions::emulator_host`]: crate::auth::FirebaseAuthClientOptions::emulator_host

use crate::{ServiceAccount, error::FirebaseError};

/// The prefix Firebase reserves for projects that exist only in the emulators.
pub(crate) const DEMO_PROJECT_PREFIX: &str = "demo-";

/// The bearer token both emulators take as an administrator's. With it the
/// Firestore emulator skips security rules, as the real service does for a
/// service account, and the Auth emulator serves the admin endpoints.
pub(crate) const OWNER_TOKEN: &str = "owner";

/// Refuses emulator mode for a project that could be a real one.
pub(crate) fn ensure_demo_project(project_id: &str) -> Result<(), FirebaseError> {
    if project_id.starts_with(DEMO_PROJECT_PREFIX) {
        return Ok(());
    }

    Err(FirebaseError::Other(anyhow::anyhow!(
        "Emulator mode is only allowed for a project whose ID starts with '{DEMO_PROJECT_PREFIX}', \
         and this one is '{project_id}'. Firebase reserves that prefix for projects that exist \
         only in the emulators."
    )))
}

/// `host:port` as the emulators' own environment variables give it, with
/// `localhost` spelled as the IPv4 loopback address: the emulators listen on
/// IPv4, and `localhost` may resolve to IPv6 first.
pub(crate) fn normalise_host(host: &str) -> String {
    let host = host
        .trim()
        .trim_start_matches("http://")
        .trim_end_matches('/');

    match host.strip_prefix("localhost") {
        Some(rest) if rest.is_empty() || rest.starts_with(':') => format!("127.0.0.1{rest}"),
        _ => host.to_string(),
    }
}

/// The service account this crate's own tests use against the emulators: the
/// project `firebase emulators:exec` runs them under (`GCLOUD_PROJECT`), or
/// `demo-fireplace`, and no credentials, because the emulators check none.
pub(crate) fn service_account_for_tests() -> ServiceAccount {
    let project_id = std::env::var("GCLOUD_PROJECT")
        .ok()
        .filter(|project_id| project_id.starts_with(DEMO_PROJECT_PREFIX))
        .unwrap_or_else(|| "demo-fireplace".to_string());

    ServiceAccount {
        project_id,
        private_key: String::new(),
        private_key_id: String::new(),
        client_email: "emulator@example.com".to_string(),
        client_id: "emulator".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_demo_project_may_use_emulator_mode() {
        assert!(ensure_demo_project("demo-fireplace").is_ok());
        assert!(ensure_demo_project("demo-").is_ok());

        for project_id in ["my-app", "demo", "my-demo-app", "Demo-app", ""] {
            let error = ensure_demo_project(project_id).unwrap_err().to_string();
            assert!(error.contains("demo-"), "{error}");
        }
    }

    #[test]
    fn localhost_becomes_the_ipv4_loopback_address() {
        assert_eq!(normalise_host("localhost:8080"), "127.0.0.1:8080");
        assert_eq!(normalise_host("localhost"), "127.0.0.1");
        assert_eq!(normalise_host("http://localhost:9099/"), "127.0.0.1:9099");
        assert_eq!(normalise_host(" 127.0.0.1:8080 "), "127.0.0.1:8080");
        assert_eq!(normalise_host("[::1]:8080"), "[::1]:8080");
        assert_eq!(
            normalise_host("localhost.example.com:1"),
            "localhost.example.com:1"
        );
        assert_eq!(normalise_host("emulator:8080"), "emulator:8080");
    }
}
