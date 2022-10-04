use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignUpResponse {
    email: String,
    #[serde(rename(deserialize = "localId"))]
    user_uid: String,
    id_token: String,
    refresh_token: String,
}
