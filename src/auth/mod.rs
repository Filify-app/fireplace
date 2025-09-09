use anyhow::Context;
use reqwest::Response;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    auth::{
        error::AuthApiErrorResponse,
        models::{BatchGetResponse, UpdateUserBody, UpdateUserValues},
    },
    error::FirebaseError,
    ServiceAccount,
};

use self::{
    credential::{ApiAuthTokenManager, UserTokenManager},
    models::{GetAccountInfoResponse, NewUser, User},
};

mod credential;
mod error;
pub mod models;
pub mod test_helpers;

pub struct FirebaseAuthClient {
    client: reqwest::Client,
    api_url: String,
    user_token_manager: UserTokenManager,
    api_auth_token_manager: ApiAuthTokenManager,
    project_id: String,
}

impl FirebaseAuthClient {
    pub fn new(service_account: ServiceAccount) -> Result<Self, FirebaseError> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .build()
            .context("Failed to create HTTP client")?;

        let credential_manager = ApiAuthTokenManager::new(service_account.clone());
        let project_id = service_account.project_id.clone();
        let token_handler = UserTokenManager::new(service_account, client.clone());

        Ok(Self {
            user_token_manager: token_handler,
            client,
            api_url: "https://identitytoolkit.googleapis.com/v1".to_string(),
            api_auth_token_manager: credential_manager,
            project_id,
        })
    }

    fn url(&self, path: impl AsRef<str>) -> String {
        format!("{}{}", self.api_url, path.as_ref())
    }

    fn project_url(&self, path: impl AsRef<str>) -> String {
        format!(
            "{}/projects/{}{}",
            self.api_url,
            &self.project_id,
            path.as_ref()
        )
    }

    async fn get_access_token(&self) -> Result<String, FirebaseError> {
        let access_token = self
            .api_auth_token_manager
            .get_access_token()
            .await
            .map_err(|e| {
                tracing::error!("Failed to get access token: {e}");
                e
            })?;

        Ok(access_token)
    }

    /// Creates a new `POST` request builder with the `Authorization` header set
    /// to an authorized admin access token.
    async fn auth_post(
        &self,
        url: impl AsRef<str>,
    ) -> Result<reqwest::RequestBuilder, FirebaseError> {
        let access_token = self.get_access_token().await?;

        let builder = self
            .client
            .post(url.as_ref())
            .header("Authorization", format!("Bearer {access_token}"));

        Ok(builder)
    }

    /// Creates a new `GET` request builder with the `Authorization` header set
    /// to an authorized admin access token.
    async fn auth_get(
        &self,
        url: impl AsRef<str>,
    ) -> Result<reqwest::RequestBuilder, FirebaseError> {
        let access_token = self.get_access_token().await?;

        let builder = self
            .client
            .get(url.as_ref())
            .header("Authorization", format!("Bearer {access_token}"));

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
    /// use fireplace::auth::models::NewUser;
    ///
    /// // Create some user so we can get a valid ID token
    /// let user_id = auth_client
    ///     .create_user(NewUser {
    ///         display_name: Some("Mario".to_string()),
    ///         email: format!("{}@example.com", Ulid::new()),
    ///         password: Ulid::new().to_string(),
    ///     })
    ///     .await?;
    ///
    /// // Generate custom token, which the "user" can use to sign into Firebase
    /// let custom_token = auth_client.create_custom_token(&user_id).await?;
    ///
    /// // Sign into Firebase to obtain an ID token
    /// let id_token = auth_client.sign_in_with_custom_token(&custom_token).await?;
    ///
    /// // Decode the ID token. If we get Ok back, we know it's valid and the
    /// // user is authenticated.
    /// let decoded_token = auth_client
    ///     .decode_id_token::<serde_json::Value>(&id_token)
    ///     .await?;
    ///
    /// assert_eq!(user_id, decoded_token["user_id"].as_str().unwrap());
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
    /// # use fireplace::auth::models::NewUser;
    /// # use serde::Deserialize;
    /// # let user_id = auth_client
    /// #     .create_user(NewUser {
    /// #         display_name: Some("Mario".to_string()),
    /// #         email: format!("{}@example.com", Ulid::new()),
    /// #         password: Ulid::new().to_string(),
    /// #     })
    /// #     .await?;
    /// # let custom_token = auth_client.create_custom_token(&user_id).await?;
    /// # let id_token = auth_client.sign_in_with_custom_token(&custom_token).await?;
    /// #
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
    ///
    /// # Examples
    ///
    /// See the first example for [`decode_id_token`](Self::decode_id_token).
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

    /// Retrieve info about a user by their user ID. Returns `None` if the user
    /// does not exist.
    ///
    /// You will also get back any custom claims that have been set on the user.
    /// See the examples in [`set_custom_user_claims`](Self::set_custom_user_claims).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::auth::models::NewUser;
    /// use ulid::Ulid;
    ///
    /// // Create a user we can fetch afterwards
    /// let email = format!("{}@example.com", Ulid::new());
    /// let user = auth_client
    ///     .create_user(NewUser {
    ///         display_name: Some("Mario".to_string()),
    ///         email: email.clone(),
    ///         password: Ulid::new().to_string(),
    ///     })
    ///     .await?;
    ///
    /// let user = auth_client.get_user(&user).await?.unwrap();
    ///
    /// assert_eq!(user.display_name, Some("Mario".to_string()));
    ///
    /// // A noteworthy thing to mention is that Firebase will turn the email
    /// // address into lowercase:
    /// assert_eq!(user.email, Some(email.to_lowercase()));
    ///
    /// // ... and there are many more fields to explore
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// If you try to fetch a user that doesn't exist, you'll get `None`:
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// assert!(auth_client.get_user("does-not-exist").await?.is_none());
    /// # Ok(())
    /// # }
    /// ```
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

    #[tracing::instrument(name = "Get all users", skip_all)]
    pub async fn get_all_users(&self) -> Result<Vec<User>, FirebaseError> {
        let base_url = self.project_url("/accounts:batchGet");

        fn make_pagination_url(
            base_url: &str,
            max_results: usize,
            next_page_token: Option<&str>,
        ) -> String {
            format!(
                "{base_url}?maxResults={max_results}{}",
                next_page_token
                    .map(|token| format!("&nextPageToken={token}"))
                    .unwrap_or_else(|| { "".to_string() })
            )
        }

        // Potential future work: make the requests concurrently by using a binary search
        // methodology. The "next page token" seems to be a user ID, but testing shows that
        // the ID does not need to exist - the results are just users after that ID
        // (lexicographically).
        //
        // So we could do something like:
        //   - Two initial requests:
        //      1) no token
        //      2) a request with next page token `a` (halfway through the alphabet of upper +
        //         lowercase letters)
        // For example, find lower and upper bounds for user IDs and divide & conquer all
        // subranges until they start overlapping.
        //
        // Keep "guessing" and combining results until we have all users. Would be slower
        // for small sets, but for large sets of users it could cut down on the sequential
        // requests significantly.

        let mut all_users = Vec::new();
        let mut next_page_token = None;
        loop {
            let url = make_pagination_url(&base_url, 1000, next_page_token.as_deref());

            let res = self
                .auth_get(url)
                .await?
                .header("Content-Type", "application/json")
                .send()
                .await
                .context("Failed to send get all users request")?;

            if !res.status().is_success() {
                return Err(response_error("Failed to get all users", res).await);
            }

            let res_body: BatchGetResponse =
                res.json().await.context("Failed to read response JSON")?;

            if let Some(mut users) = res_body.users {
                all_users.append(&mut users);
            }

            next_page_token = res_body.next_page_token.map(|t| t.to_string());

            if next_page_token.is_none() {
                break;
            }
        }

        Ok(all_users)
    }

    /// Creates a new user in Firebase Auth using the email/password provider.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::{auth::models::NewUser, error::FirebaseError};
    /// use ulid::Ulid;
    ///
    /// let new_user = NewUser {
    ///     display_name: Some("Mario".to_string()),
    ///     email: format!("{}@example.com", Ulid::new()),
    ///     password: Ulid::new().to_string(),
    /// };
    ///
    /// // When we create the user, we get back their unique user ID
    /// let user_id = auth_client.create_user(new_user.clone()).await?;
    ///
    /// println!("Created user with ID '{}'", user_id);
    ///
    /// // If we attempt to create another user with the same email, Firebase
    /// // will complain
    /// let create_again_result = auth_client.create_user(new_user).await;
    ///
    /// assert!(matches!(
    ///     create_again_result,
    ///     Err(FirebaseError::EmailAlreadyExists)
    /// ));
    /// # Ok(())
    /// # }
    /// ```
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

    /// Updates a user's attributes in Firebase Auth, such as email or display name.
    ///
    /// This function allows you to update specific fields of a user. Passing `None` for a field
    /// will remove it. Only the provided fields will be modified; others remain unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), fireplace::error::FirebaseError> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    /// use fireplace::auth::models::{NewUser, UpdateUserValues};
    /// use ulid::Ulid;
    ///
    /// let user_id = auth_client
    ///     .create_user(NewUser {
    ///         display_name: Some("Julius Caesar".to_string()),
    ///         email: format!("caesar@rome{}.it", Ulid::new()),
    ///         password: "venividivici".to_string(),
    ///     })
    ///     .await?;
    ///
    /// // Give a new value for the email
    /// let new_email = format!("caesar@deceased{}.it", Ulid::new());
    ///
    /// // Pass `None` to delete a field
    /// let new_display_name: Option<String> = None;
    ///
    /// let res = auth_client
    ///     .update_user(
    ///         &user_id,
    ///         UpdateUserValues::new()
    ///             .email(&new_email)
    ///             .display_name(new_display_name),
    ///     )
    ///     .await?;
    ///
    /// assert_eq!(res.email, Some(new_email.to_lowercase()));
    /// assert_eq!(res.display_name, None);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(name = "Update user", skip_all, fields(user_id = %user_id.as_ref()))]
    pub async fn update_user(
        &self,
        user_id: impl AsRef<str>,
        updated_values: UpdateUserValues,
    ) -> Result<User, FirebaseError> {
        let body_values = UpdateUserBody::from_values(user_id.as_ref(), updated_values);
        let body =
            serde_json::to_string(&body_values).context("Failed to serialize updated values")?;

        let res = self
            .auth_post(self.url("/accounts:update"))
            .await?
            .body(body)
            .send()
            .await
            .context("Failed to send update user request")?;

        if !res.status().is_success() {
            let err = res
                .json::<AuthApiErrorResponse>()
                .await
                .context("Failed to read error response JSON")?
                .into();

            tracing::error!("Failed to update user: {err}");

            return Err(err);
        }

        let res_body: User = res.json().await.context("Failed to read response JSON")?;

        tracing::info!("Updated user with id '{}'", &res_body.uid);

        Ok(res_body)
    }

    /// Signs into Firebase with a custom generated token, which you can get
    /// from [`create_custom_token`](Self::create_custom_token). Returns an ID
    /// token for Firebase.
    ///
    /// # Examples
    ///
    /// See the first example for [`decode_id_token`](Self::decode_id_token).
    #[tracing::instrument(name = "Sign in with custom token", skip(self, custom_token))]
    pub async fn sign_in_with_custom_token(
        &self,
        custom_token: impl AsRef<str>,
    ) -> Result<String, FirebaseError> {
        tracing::debug!("Signing in with custom token");

        let body = serde_json::json!({
            "token": custom_token.as_ref(),
            "returnSecureToken": true,
        });

        let res = self
            .auth_post(self.url("/accounts:signInWithCustomToken"))
            .await?
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send sign-in request")?;

        if !res.status().is_success() {
            return Err(response_error("Failed to get user", res).await);
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SignInResponse {
            id_token: String,
        }

        let res_body: SignInResponse = res.json().await.context("Failed to read response JSON")?;

        Ok(res_body.id_token)
    }

    /// Set custom attributes on a user. The attributes can be anything JSON-
    /// serializable. This will overwrite any existing attributes competely.
    ///
    /// The fields that you set as custom claims will show up in the ID token
    /// claims. This can, for example, be useful for access-control. Note that
    /// users need to re-authenticate for the custom claims to appear in the ID
    /// token.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), anyhow::Error> {
    /// # let auth_client = fireplace::auth::test_helpers::initialise()?;
    ///
    /// use fireplace::auth::models::NewUser;
    /// use serde::{Deserialize, Serialize};
    /// use ulid::Ulid;
    ///
    /// // Create a user we can set some claims on
    /// let user_id = auth_client
    ///     .create_user(NewUser {
    ///         display_name: Some("Mario".to_string()),
    ///         email: format!("{}@example.com", Ulid::new()),
    ///         password: Ulid::new().to_string(),
    ///     })
    ///     .await?;
    ///
    /// // Initially, the user will have no claims
    /// let user = auth_client.get_user(&user_id).await?.unwrap();
    /// assert_eq!(user.custom_claims, serde_json::Value::Null);
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct CustomClaims {
    ///     #[serde(default)]
    ///     roles: Vec<String>,
    /// }
    ///
    /// // Set some custom claims
    /// auth_client
    ///     .set_custom_user_claims(
    ///         &user_id,
    ///         CustomClaims {
    ///             roles: vec!["superhero".to_string()],
    ///         },
    ///     )
    ///     .await?;
    ///
    /// // Now, the user should have those claims as a JSON value
    /// let user = auth_client.get_user(&user_id).await?.unwrap();
    /// let custom_claims: CustomClaims = serde_json::from_value(user.custom_claims)?;
    ///
    /// assert_eq!(custom_claims.roles, vec!["superhero"]);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(name = "Set custom user claims", skip(self, user_id, new_claims))]
    pub async fn set_custom_user_claims<C: Serialize>(
        &self,
        user_id: &str,
        new_claims: C,
    ) -> Result<(), FirebaseError> {
        let custom_claims =
            serde_json::to_string(&new_claims).context("Failed to serialize claims")?;

        let body = serde_json::json!({
            "localId": user_id,
            "customAttributes": custom_claims,
        });

        let res = self
            .auth_post(self.url("/accounts:update"))
            .await?
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send custom claims request")?;

        if !res.status().is_success() {
            return Err(response_error("Failed to set custom user claims", res).await);
        }

        tracing::debug!("Set custom claims for user '{}'", user_id);

        Ok(())
    }
}

async fn response_error(msg: &'static str, res: Response) -> FirebaseError {
    let status = res.status();
    let body = res.text().await.unwrap_or_default();

    let err = anyhow::anyhow!("{} (status: {}): {}", msg, status, body).into();

    tracing::error!("{:?}'", &err);

    err
}
