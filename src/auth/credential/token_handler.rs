use anyhow::Context;
use jsonwebtoken::{get_current_timestamp, Algorithm, DecodingKey, Validation};
use serde::{de::DeserializeOwned, Serialize};

use super::public_keys::PublicKeys;

use crate::ServiceAccount;

const FIREBASE_AUDIENCE: &str =
    "https://identitytoolkit.googleapis.com/google.identity.identitytoolkit.v1.IdentityToolkit";

pub struct UserTokenManager {
    public_keys: PublicKeys,
    service_account: ServiceAccount,
}

impl UserTokenManager {
    pub fn new(service_account: ServiceAccount, http_client: reqwest::Client) -> Self {
        Self {
            public_keys: PublicKeys::new(http_client),
            service_account,
        }
    }

    /// Verifies an ID token based on the docs at <https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library>
    ///
    /// Fails if the token is in a bad format, expired, not issued for this
    /// project, or if the signature is invalid.
    pub async fn decode_id_token<C: DeserializeOwned>(
        &self,
        token: &str,
    ) -> Result<C, anyhow::Error> {
        let header = jsonwebtoken::decode_header(token)?;

        if header.alg != jsonwebtoken::Algorithm::RS256 {
            anyhow::bail!("Invalid ID token JWT algorithm");
        }

        let public_key_id = header
            .kid
            .context("ID token is missing public key ID in header")?;

        let public_key = self
            .public_keys
            .get(&public_key_id)
            .await?
            .context("Unrecognized public key in header of ID token")?;

        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&[&self.service_account.project_id]);
        validation.set_issuer(&[&format!(
            "https://securetoken.google.com/{}",
            &self.service_account.project_id
        )]);

        let decoded = jsonwebtoken::decode(
            token,
            &DecodingKey::from_rsa_pem(public_key.as_ref())
                .context("Invalid public key format in ID token")?,
            &validation,
        )?;

        Ok(decoded.claims)
    }

    /// Creates and signs a custom token for a user ID, which the user can use
    /// to authenticate against Firebase services.
    ///
    /// See the official [Firebase Auth docs for creating custom tokens](https://firebase.google.com/docs/auth/admin/create-custom-tokens#create_custom_tokens_using_a_third-party_jwt_library>).
    pub async fn create_custom_token(&self, uid: &str) -> Result<String, anyhow::Error> {
        #[derive(Serialize)]
        struct CustomTokenClaims<'a> {
            aud: &'a str,
            iat: u64,
            exp: u64,
            iss: &'a str,
            sub: &'a str,
            uid: &'a str,
        }

        let header = jsonwebtoken::Header::new(Algorithm::RS256);

        let issued_at_time = get_current_timestamp();
        let expires_at = issued_at_time + (60 * 60);

        let claims = CustomTokenClaims {
            iss: &self.service_account.client_email,
            sub: &self.service_account.client_email,
            aud: FIREBASE_AUDIENCE,
            iat: issued_at_time,
            exp: expires_at,
            uid,
        };

        let encoding_key =
            jsonwebtoken::EncodingKey::from_rsa_pem(self.service_account.private_key.as_bytes())
                .context("Failed to create JWT encoding key from the given private key")?;

        let jwt = jsonwebtoken::encode(&header, &claims, &encoding_key)
            .context("Failed to create custom token JWT")?;

        Ok(jwt)
    }
}
