use anyhow::Context;
use jsonwebtoken::{Algorithm, get_current_timestamp};
use serde::Serialize;

use crate::{ServiceAccount, error::FirebaseError};

#[derive(Clone)]
pub struct FirestoreTokenProvider {
    service_account: ServiceAccount,
    current_token: Option<Token>,
}

#[derive(Clone)]
struct Token {
    jwt: String,
    /// The timestamp at which the token expires. Represented as seconds since
    /// the UNIX epoch in accordance with the [Firebase API docs](https://firebase.google.com/docs/auth/admin/create-custom-tokens#create_custom_tokens_using_a_third-party_jwt_library).
    expires_at: u64,
}

impl FirestoreTokenProvider {
    pub fn new(service_account: ServiceAccount) -> Self {
        Self {
            service_account,
            current_token: None,
        }
    }

    pub fn get_token(&mut self) -> Result<String, FirebaseError> {
        match &self.current_token {
            Some(token) if token.expires_at > get_current_timestamp() => Ok(token.jwt.clone()),
            _ => {
                let token = create_jwt(&self.service_account)?;
                let jwt = token.jwt.clone();
                self.current_token = Some(token);
                Ok(jwt)
            }
        }
    }
}

fn create_jwt(service_account: &ServiceAccount) -> Result<Token, anyhow::Error> {
    let mut header = jsonwebtoken::Header::new(Algorithm::RS256);
    header.kid = Some(service_account.private_key_id.clone());

    let valid_duration = 60 * 60; // the token will be valid for 60 minutes
    let expiry_buffer = 5 * 60; // but we will create a new token after 55 minutes just to be sure

    let issued_at_time = get_current_timestamp();
    let claims = JwtClaims {
        iss: &service_account.client_email,
        sub: &service_account.client_email,
        // TODO: This is something I had to find in some random place. The official aud URL
        // doesn't work. How to fix?
        aud: "https://firestore.googleapis.com/",
        iat: issued_at_time,
        exp: issued_at_time + valid_duration,
        uid: &service_account.client_id,
    };

    let encoding_key =
        jsonwebtoken::EncodingKey::from_rsa_pem(service_account.private_key.as_ref())
            .context("Failed to create JWT encoding key from the given private key")?;

    let jwt =
        jsonwebtoken::encode(&header, &claims, &encoding_key).context("Failed to create JWT")?;

    Ok(Token {
        jwt,
        expires_at: claims.exp - expiry_buffer,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatically_regenerates_token_when_expired() {
        let service_account = ServiceAccount {
            project_id: "test-project".to_string(),
            private_key: RANDOM_RSA_KEY.to_string(),
            private_key_id: "some private key id here".to_string(),
            client_email: "some client email here".to_string(),
            client_id: "some client id here".to_string(),
        };

        let mut token_provider = FirestoreTokenProvider::new(service_account);

        let initial_token = token_provider.get_token().unwrap();

        // We have to wait for at least a second or else the regenerated token
        // will be the same as the original token (since both tokens will have
        // the same issued-at time).
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Simulate that some time has passed (but the token is still valid).
        token_provider.current_token.as_mut().unwrap().expires_at -= 50 * 60;
        let reused_token = token_provider.get_token().unwrap();
        assert_eq!(initial_token, reused_token);

        // Simulate that enough time has passed to expire the token.
        token_provider.current_token.as_mut().unwrap().expires_at -= 10 * 60;
        let new_token = token_provider.get_token().unwrap();
        assert_ne!(initial_token, new_token);
    }

    // Don't worry, this is just a random RSA private key that has been
    // generated for testing purposes.
    const RANDOM_RSA_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----\n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\n-----END RSA PRIVATE KEY-----";
}
