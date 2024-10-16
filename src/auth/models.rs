use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct GetAccountInfoResponse {
    pub users: Option<Vec<User>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "localId")]
    pub uid: String,
    pub password_hash: Option<String>,
    pub password_updated_at: Option<u64>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub phone_number: Option<String>,
    pub display_name: Option<String>,
    pub photo_url: Option<String>,
    pub disabled: Option<bool>,
    pub salt: Option<String>,
    #[serde(
        default,
        rename = "customAttributes",
        deserialize_with = "deserialize_custom_attributes"
    )]
    pub custom_claims: serde_json::Value,
    pub valid_since: Option<String>,
    pub tenant_id: Option<String>,
    // pub provider_user_info: Option<Vec<ProviderUserInfo>>,
    // pub mfaInfo: Option<Vec<MultiFactorInfo>>,
    pub created_at: Option<String>,
    pub last_login_at: Option<String>,
    pub last_refresh_at: Option<String>,
    #[serde(flatten)]
    pub other: serde_json::Value,
}

fn deserialize_custom_attributes<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr + Default,
    T::Err: std::fmt::Display,
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    let t = s
        .map(|s| T::from_str(&s).map_err(serde::de::Error::custom))
        .transpose()?
        .unwrap_or_default();
    Ok(t)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    pub display_name: Option<String>,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserValues {
    display_name: Option<Option<String>>,
    email: Option<String>,
    password: Option<String>,
}

impl UpdateUserValues {
    /// Create an empty instance that updates no fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the display name of the user. If `None` is passed, the display name will be removed.
    pub fn display_name(mut self, display_name: Option<String>) -> Self {
        self.display_name = Some(display_name);
        self
    }

    /// Update the user's email.
    pub fn email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    /// Update the user's password.
    pub fn password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserBody<'a> {
    local_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    delete_attribute: Vec<String>,
}

impl<'a> UpdateUserBody<'a> {
    pub fn from_values(user_id: &'a str, values: UpdateUserValues) -> Self {
        // We need to specify a list of attributes to delete explicitly according to
        // the Firebase Node.js Admin SDK implementation: https://github.com/firebase/firebase-admin-node/blob/f1c55238a885a76b5225fe5bdaa580c7ae1cc8a4/src/auth/auth-api-request.ts#L1418-L1436
        let mut delete_attribute = Vec::new();

        if let Some(None) = values.display_name {
            delete_attribute.push("DISPLAY_NAME".to_string());
        }

        Self {
            local_id: user_id,
            display_name: values.display_name.flatten(),
            email: values.email,
            password: values.password,
            delete_attribute,
        }
    }
}
