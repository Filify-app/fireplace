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
