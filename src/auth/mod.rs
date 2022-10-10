use anyhow::Context;
use serde::de::DeserializeOwned;

use crate::error::FirebaseError;

use self::{error::AuthApiErrorResponse, models::SignUpResponse};

mod error;
pub mod models;
pub mod test_helpers;
mod token;

pub struct FirebaseAuthClient {
    client: reqwest::Client,
    api_url: String,
    token_handler: token::TokenHandler,
}

impl FirebaseAuthClient {
    pub fn new(project_id: String, api_key: &str) -> Result<Self, FirebaseError> {
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
            token_handler: token::TokenHandler::new(project_id, client.clone()),
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
    /// # use fireplace::error::FirebaseError;
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
                .context("Failed to read error response JSON")?
                .into();

            tracing::error!("Failed with '{}'", &err);

            Err(err)
        }
    }

    /// Decodes an ID token and returns its claims. Only succeeds if the token
    /// is valid. The token is valid if it:
    ///
    /// - Is not expired
    /// - Is issued for this Firebase project
    /// - Has a valid digital signature from Google
    ///
    /// The [Firebase API docs] list further requirements.
    ///
    /// # Generic parameters
    ///
    /// The generic type parameter `C` is the format of the decoded JWT claims
    /// that will be used for deserialization. See the examples below.
    ///
    /// # Examples
    ///
    /// A valid token:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # use ulid::Ulid;
    /// # let mut auth_client = fireplace::auth::test_helpers::initialise()?;
    /// // Create some user so we can get a valid ID token
    /// let signed_up_user = auth_client
    ///     .sign_up_with_email_and_password(format!("{}@example.com", Ulid::new()), Ulid::new())
    ///     .await?;
    ///
    /// // Decode the ID token. If we get Ok back, we know it's valid and the
    /// // user is authenticated.
    /// let decoded_token = auth_client
    ///     .decode_id_token::<serde_json::Value>(&signed_up_user.id_token)
    ///     .await?;
    ///
    /// assert_eq!(signed_up_user.user_uid, decoded_token["user_id"].as_str().unwrap());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// An invalid token will result in an error:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # use ulid::Ulid;
    /// # let mut auth_client = fireplace::auth::test_helpers::initialise()?;
    /// // Some invalid ID token. It is expired, and it might be issued for a
    /// // different Firebase project.
    /// let id_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6IjU4NWI5MGI1OWM2YjM2ZDNjOTBkZjBlOTEwNDQ1M2U2MmY4ODdmNzciLCJ0eXAiOiJKV1QifQ.eyJpc3MiOiJodHRwczovL3NlY3VyZXRva2VuLmdvb2dsZS5jb20vcnVzdC1hZG1pbi1zZGstdGVzdCIsImF1ZCI6InJ1c3QtYWRtaW4tc2RrLXRlc3QiLCJhdXRoX3RpbWUiOjE2NjQ5OTUwNjcsInVzZXJfaWQiOiJIRnRxZ0NQc0hTTTF5SngwUnVaY0ZXbVQ5TEMzIiwic3ViIjoiSEZ0cWdDUHNIU00xeUp4MFJ1WmNGV21UOUxDMyIsImlhdCI6MTY2NDk5NTA2NywiZXhwIjoxNjY0OTk4NjY3LCJlbWFpbCI6ImZzYWZhQHRlc3RwLmFwcCIsImVtYWlsX3ZlcmlmaWVkIjpmYWxzZSwiZmlyZWJhc2UiOnsiaWRlbnRpdGllcyI6eyJlbWFpbCI6WyJmc2FmYUB0ZXN0cC5hcHAiXX0sInNpZ25faW5fcHJvdmlkZXIiOiJwYXNzd29yZCJ9fQ.ImphBsbuXJOMKyZF21YIK0PQ4ZFwPDDfJ56wW1cJkKBUhGUICW9zNv2WgCuZ03XdfexYcGabUjetruOQBx9c9eSJsPZQdAblNYk9vcBbmpaxya55HNkSbp2ZfX5S_ReUSekjiGsd53qfRLOTHxu4m-LGddE2_lfz6Mx2IAf9ij6JjU-uc5w5klmT3OAUkxUBpPyAcocwHU0WqXuOYDBo-WRL8hC2CTgQ8o0Mo-wHBsIZ_IU_SkIHG7xl2oq91Gm7q227KX7j5LnNaOgM3GuCOajPzzyKzTKAcX2pCKlkyR1bQHuefzuyPF_RME0jroOuHZm031uW_v4rnMWO3HtmDw";
    /// let decode_result = auth_client
    ///     .decode_id_token::<serde_json::Value>(id_token)
    ///     .await;
    ///
    /// assert!(decode_result.is_err());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Deserializing to your own format:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # use ulid::Ulid;
    /// # let mut auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use serde::Deserialize;
    ///
    /// let id_token = auth_client
    ///     .sign_up_with_email_and_password(format!("{}@example.com", Ulid::new()), Ulid::new())
    ///     .await?
    ///     .id_token;
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct Claims {
    ///     user_id: String,
    ///     email: String,
    ///     firebase: FirebaseClaims,
    /// }
    ///
    /// #[derive(Debug, Deserialize)]
    /// struct FirebaseClaims {
    ///     sign_in_provider: String,
    /// }
    ///
    /// // We can make our own claims type and deserialize into that
    /// let claims = auth_client.decode_id_token::<Claims>(&id_token).await?;
    ///
    /// // Or we can just use serde_json::Value:
    /// let claims_json = auth_client
    ///     .decode_id_token::<serde_json::Value>(&id_token)
    ///     .await?;
    ///
    /// assert_eq!(claims.user_id, claims_json["user_id"].as_str().unwrap());
    /// assert_eq!(claims.email, claims_json["email"].as_str().unwrap());
    /// assert_eq!(
    ///     claims.firebase.sign_in_provider,
    ///     claims_json["firebase"]["sign_in_provider"]
    ///         .as_str()
    ///         .unwrap()
    /// );
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [Firebase API docs]: https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library
    #[tracing::instrument(name = "Decode ID token", skip(self, token))]
    pub async fn decode_id_token<C: DeserializeOwned>(
        &mut self,
        token: &str,
    ) -> Result<C, FirebaseError> {
        let id_token_claims = self
            .token_handler
            .decode_id_token(token.as_ref())
            .await
            .map_err(FirebaseError::ValidateTokenError)?;

        Ok(id_token_claims)
    }
}
