use anyhow::Context;
use jsonwebtoken::{DecodingKey, Validation};
use serde::de::DeserializeOwned;

use super::public_keys::PublicKeys;

use crate::token::ServiceAccount;

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
}
