use anyhow::Context;

use crate::error::FirebaseError;

use self::{error::AuthApiErrorResponse, models::SignUpResponse};

mod error;
pub mod models;
pub mod test_helpers;

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

    /// Creates a new user with an email address and password.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use ulid::Ulid;
    /// # let auth_client = fireplace::auth::test_helpers::initialise().unwrap();
    /// #
    /// // Generate some random email address and password
    /// let email = format!("{}@example.com", Ulid::new());
    /// let password = Ulid::new().to_string();
    ///
    /// // Sign up
    /// let new_user = auth_client
    ///     .sign_up_with_email_and_password(&email, &password)
    ///     .await
    ///     .unwrap();
    ///
    /// // We get back info about the new user, including its ID and some
    /// // tokens the user can use to authenticate with Firebase.
    /// println!("Created user with id '{}'", &new_user.user_uid);
    ///
    /// // It's worth noting that Firebase Auth turns the email into lowercase.
    /// assert_eq!(email.to_lowercase(), new_user.email);
    ///
    /// // You cannot create two users with the same email
    /// let another_new_user_result = auth_client
    ///     .sign_up_with_email_and_password(&email, &password)
    ///     .await;
    ///
    /// assert!(matches!(
    ///     another_new_user_result,
    ///     Err(FirebaseError::EmailAlreadyExists)
    /// ));
    /// # }
    /// ```
    #[tracing::instrument(name = "Sign up with email", skip(self, email, password))]
    pub async fn sign_up_with_email_and_password(
        &self,
        email: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<SignUpResponse, FirebaseError> {
        let email = email.into();

        tracing::info!("Signing up user with email '{}'", &email);

        let body = serde_json::json!({
            "email": email,
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

            tracing::info!("Created user with id '{}'", &new_user.user_uid);

            Ok(new_user)
        } else {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to read response JSON")?
                .into();

            tracing::error!("Failed with '{}'", &err);

            Err(err)
        }
    }
}
