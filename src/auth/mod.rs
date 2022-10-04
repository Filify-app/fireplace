use anyhow::Context;

use crate::error::FirebaseError;

use self::{error::AuthApiError, models::SignUpResponse};

mod error;
mod models;

pub struct FirebaseAuthClient {
    client: reqwest::Client,
    api_url: String,
}

impl FirebaseAuthClient {
    pub fn new(api_key: &str) -> Result<Self, FirebaseError> {
        let mut default_headers = reqwest::header::HeaderMap::new();

        let mut api_key_header =
            reqwest::header::HeaderValue::from_str(api_key).context("Invalid API key")?;
        api_key_header.set_sensitive(true);
        default_headers.insert("X-goog-api-key", api_key_header);

        let client = reqwest::Client::builder()
            .https_only(true)
            .default_headers(default_headers)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            api_url: "https://identitytoolkit.googleapis.com/v1/accounts".to_string(),
        })
    }

    fn url(&self, path: impl AsRef<str>) -> String {
        format!("{}:{}", self.api_url, path.as_ref())
    }

    pub async fn sign_up_with_email_and_password(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SignUpResponse, FirebaseError> {
        let body = serde_json::json!({
            "email": email.into(),
            "password": password.into(),
            "returnSecureToken": true
        });

        let res = self
            .client
            .post(self.url("signUp"))
            .body(body.to_string())
            .send()
            .await
            .context("Failed to sign up user")?;

        if res.status().is_success() {
            let new_user: SignUpResponse =
                res.json().await.context("Failed to read response JSON")?;

            Ok(new_user)
        } else {
            res.json::<AuthApiError>()
                .await
                .context("Failed to read response JSON")?
                .into()
        }
    }
}

// TODO:
// - test happy path
// - test unhappy path
// - doctest and doc explanation
