use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignUpResponse {
    /// The newly created user's email address. Note that Firebase Auth will
    /// turn the email into lowercase.
    pub email: String,
    #[serde(rename(deserialize = "localId"))]
    pub user_uid: String,
    pub id_token: String,
    pub refresh_token: String,
}
