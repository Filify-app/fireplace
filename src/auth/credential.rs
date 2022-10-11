use anyhow::Context;
use jsonwebtoken::{get_current_timestamp, Algorithm, EncodingKey};
use serde::{Deserialize, Serialize};

use crate::token::ServiceAccount;

const GOOGLE_TOKEN_AUDIENCE: &str = "https://accounts.google.com/o/oauth2/token";
const GOOGLE_AUTH_TOKEN_HOST: &str = "accounts.google.com";
const GOOGLE_AUTH_TOKEN_PATH: &str = "/o/oauth2/token";

pub struct ServiceAccountCredentialManager {
    service_account: ServiceAccount,
    http_client: reqwest::Client,
}

impl ServiceAccountCredentialManager {
    pub fn new(service_account: ServiceAccount) -> Self {
        Self {
            service_account,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn get_access_token(&self) -> Result<String, anyhow::Error> {
        let jwt = self.create_auth_jwt()?;

        let post_data = format!(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion={}",
            jwt
        );

        let url = format!(
            "https://{}{}",
            GOOGLE_AUTH_TOKEN_HOST, GOOGLE_AUTH_TOKEN_PATH
        );

        let res = self
            .http_client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(post_data)
            .send()
            .await
            .context("Failed to send auth token request to Google")?;

        anyhow::ensure!(
            res.status().is_success(),
            "Failed to get auth token from Google (status {}): {}",
            res.status(),
            res.text().await.unwrap_or_default()
        );

        let res_body = res
            .json::<AccessTokenResponse>()
            .await
            .context("Failed to read auth token response from Google")?;

        anyhow::ensure!(
            res_body.token_type == "Bearer",
            "Google did not return a Bearer token"
        );

        Ok(res_body.access_token)
    }

    fn create_auth_jwt(&self) -> Result<String, anyhow::Error> {
        let scope = [
            "https://www.googleapis.com/auth/cloud-platform",
            "https://www.googleapis.com/auth/firebase.database",
            "https://www.googleapis.com/auth/firebase.messaging",
            "https://www.googleapis.com/auth/identitytoolkit",
            "https://www.googleapis.com/auth/userinfo.email",
        ]
        .join(" ");

        let issued_at_time = get_current_timestamp();
        let expires_at = issued_at_time + (60 * 60);

        let claims = Claims {
            scope: &scope,
            aud: GOOGLE_TOKEN_AUDIENCE,
            iss: &self.service_account.client_email,
            iat: issued_at_time,
            exp: expires_at,
        };

        let header = jsonwebtoken::Header::new(Algorithm::RS256);
        let encoding_key =
            EncodingKey::from_rsa_pem(self.service_account.private_key.as_bytes())
                .context("Failed to create JWT encoding key from the given private key")?;

        let jwt = jsonwebtoken::encode(&header, &claims, &encoding_key)
            .context("Failed to encode JWT")?;

        Ok(jwt)
    }
}

#[derive(Debug, Serialize)]
struct Claims<'a> {
    scope: &'a str,
    aud: &'a str,
    iss: &'a str,
    exp: u64,
    iat: u64,
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}
