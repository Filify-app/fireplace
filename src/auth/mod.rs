use anyhow::Context;
use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize};

use crate::{auth::error::AuthApiErrorResponse, error::FirebaseError, ServiceAccount};

use self::{
    credential::{ApiAuthTokenManager, UserTokenManager},
    models::{GetAccountInfoResponse, NewUser, User},
};

pub mod credential;
mod error;
pub mod models;
pub mod test_helpers;

pub struct FirebaseAuthClient {
    client: reqwest::Client,
    api_url: String,
    user_token_manager: UserTokenManager,
    api_auth_token_manager: ApiAuthTokenManager,
}

impl FirebaseAuthClient {
    pub fn new(service_account: ServiceAccount) -> Result<Self, FirebaseError> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .build()
            .context("Failed to create HTTP client")?;

        let credential_manager = ApiAuthTokenManager::new(service_account.clone());
        let token_handler = UserTokenManager::new(service_account, client.clone());

        Ok(Self {
            user_token_manager: token_handler,
            client,
            api_url: "https://identitytoolkit.googleapis.com/v1".to_string(),
            api_auth_token_manager: credential_manager,
        })
    }

    fn url(&self, path: impl AsRef<str>) -> String {
        format!("{}{}", self.api_url, path.as_ref())
    }

    /// Creates a new `POST` request builder with the `Authorization` header set
    /// to an authorized admin access token.
    async fn auth_post(
        &self,
        url: impl AsRef<str>,
    ) -> Result<reqwest::RequestBuilder, FirebaseError> {
        let access_token = self
            .api_auth_token_manager
            .get_access_token()
            .await
            .map_err(|e| {
                tracing::error!("Failed to get access token: {}", e);
                e
            })?;

        let builder = self
            .client
            .post(url.as_ref())
            .header("Authorization", format!("Bearer {}", access_token));

        Ok(builder)
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
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
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
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
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
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
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
        &self,
        token: &str,
    ) -> Result<C, FirebaseError> {
        let id_token_claims = self
            .user_token_manager
            .decode_id_token(token)
            .await
            .map_err(FirebaseError::ValidateTokenError)?;

        Ok(id_token_claims)
    }

    /// Create a custom token for a user, which can then be used to sign into
    /// Firebase.
    #[tracing::instrument(name = "Create custom token", skip(self, user_id))]
    pub async fn create_custom_token(
        &self,
        user_id: impl AsRef<str>,
    ) -> Result<String, FirebaseError> {
        let user_id = user_id.as_ref();

        tracing::debug!("Creating custom token for user '{}'", user_id);

        let id_token_claims = self.user_token_manager.create_custom_token(user_id).await?;

        Ok(id_token_claims)
    }

    #[tracing::instrument(name = "Get user", skip(self, user_id))]
    pub async fn get_user(&self, user_id: impl AsRef<str>) -> Result<Option<User>, FirebaseError> {
        let user_id = user_id.as_ref();

        let body = serde_json::json!({
            "localId": [user_id],
        });

        tracing::debug!("Retrieving user with ID '{}'", user_id);

        let res = self
            .auth_post(self.url("/accounts:lookup"))
            .await?
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send get user request")?;

        if !res.status().is_success() {
            return Err(response_error("Failed to get user", res).await);
        }

        let res_body: GetAccountInfoResponse =
            res.json().await.context("Failed to read response JSON")?;
        let user = res_body.users.and_then(|mut users| users.pop());

        Ok(user)
    }

    #[tracing::instrument(name = "Create user", skip(self, new_user))]
    pub async fn create_user(&self, new_user: NewUser) -> Result<String, FirebaseError> {
        let body = serde_json::to_string(&new_user).context("Failed to serialize new user")?;

        let res = self
            .auth_post(self.url("/accounts:signUp"))
            .await?
            .body(body)
            .send()
            .await
            .context("Failed to send create user request")?;

        if !res.status().is_success() {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to read error response JSON")?
                .into();

            tracing::error!("Failed to create user: {}", &err);

            return Err(err);
        }

        #[derive(Deserialize)]
        struct SignUpResponse {
            #[serde(rename = "localId")]
            uid: String,
        }

        let res_body: SignUpResponse = res.json().await.context("Failed to read response JSON")?;

        tracing::info!("Created user with id '{}'", &res_body.uid);

        Ok(res_body.uid)
    }
}

async fn response_error(msg: &'static str, res: Response) -> FirebaseError {
    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    let err = anyhow::anyhow!("{} (status: {}): {}", msg, status, body).into();

    tracing::error!("{:?}'", &err);

    err
}
