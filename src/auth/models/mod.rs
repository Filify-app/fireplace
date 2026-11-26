use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

mod update_user;

pub use update_user::*;

#[derive(Debug, Deserialize)]
pub(crate) struct GetAccountInfoResponse {
    pub users: Option<Vec<User>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchGetResponse {
    pub users: Option<Vec<User>>,
    pub next_page_token: Option<String>,
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
    /// Left out of the request when `None`. Google reads a `null` as no name,
    /// but the Auth emulator refuses a `displayName` that is not a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub email: String,
    pub password: String,
}

#[cfg(test)]
mod tests {
    use super::NewUser;

    #[test]
    fn a_new_user_without_a_display_name_sends_no_display_name() {
        let unnamed = NewUser {
            display_name: None,
            email: "mario@example.com".to_string(),
            password: "its-a-me".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&unnamed).unwrap(),
            serde_json::json!({ "email": "mario@example.com", "password": "its-a-me" })
        );

        let named = NewUser {
            display_name: Some("Mario".to_string()),
            ..unnamed
        };
        assert_eq!(
            serde_json::to_value(&named).unwrap(),
            serde_json::json!({
                "displayName": "Mario",
                "email": "mario@example.com",
                "password": "its-a-me",
            })
        );
    }
}
