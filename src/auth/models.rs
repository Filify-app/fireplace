use serde::{Deserialize, Serialize};

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
    pub custom_attributes: Option<String>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    pub display_name: Option<String>,
    pub email: String,
    pub password: String,
}
