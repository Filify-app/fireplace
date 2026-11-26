use anyhow::Context;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, get_current_timestamp};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::public_keys::PublicKeys;

use crate::ServiceAccount;

const FIREBASE_AUDIENCE: &str =
    "https://identitytoolkit.googleapis.com/google.identity.identitytoolkit.v1.IdentityToolkit";

/// The seconds an expiry time may lie in the past and still pass, for clocks
/// that disagree. The same leeway `jsonwebtoken` gives a signed token.
const EXPIRY_LEEWAY_SECONDS: u64 = 60;

pub struct UserTokenManager {
    public_keys: PublicKeys,
    service_account: ServiceAccount,
    /// Whether tokens come from, and go to, the Auth emulator. See
    /// [`UserTokenManager::for_emulator`].
    emulator: bool,
}

impl UserTokenManager {
    pub fn new(service_account: ServiceAccount, http_client: reqwest::Client) -> Self {
        Self {
            public_keys: PublicKeys::new(http_client),
            service_account,
            emulator: false,
        }
    }

    /// A manager for the Auth emulator, whose tokens carry no signature.
    ///
    /// The emulator issues ID tokens with `alg: none`, so there is nothing to
    /// verify them against: an ID token is accepted on its claims alone
    /// (project, issuer, expiry, subject). Custom tokens are created unsigned
    /// as well, which the emulator accepts and which needs no private key.
    ///
    /// Anyone can write such a token, so this must never serve a real
    /// project. The caller guarantees that: `FirebaseAuthClient` only builds
    /// this for a `demo-` project.
    pub fn for_emulator(service_account: ServiceAccount, http_client: reqwest::Client) -> Self {
        Self {
            emulator: true,
            ..Self::new(service_account, http_client)
        }
    }

    fn issuer(&self) -> String {
        format!(
            "https://securetoken.google.com/{}",
            self.service_account.project_id
        )
    }

    /// Verifies an ID token based on the docs at <https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library>
    ///
    /// Fails if the token is in a bad format, expired, not issued for this
    /// project, or if the signature is invalid.
    pub async fn decode_id_token<C: DeserializeOwned>(
        &self,
        token: &str,
    ) -> Result<C, anyhow::Error> {
        if self.emulator {
            return self.decode_unsigned_id_token(token);
        }

        let header = jsonwebtoken::decode_header(token)?;

        if header.alg != jsonwebtoken::Algorithm::RS256 {
            anyhow::bail!("Invalid ID token JWT algorithm");
        }

        let public_key_id = header
            .kid
            .context("ID token is missing public key ID in header")?;

        let public_key = self
            .public_keys
            .get(&public_key_id)
            .await?
            .context("Unrecognized public key in header of ID token")?;

        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_audience(&[&self.service_account.project_id]);
        validation.set_issuer(&[&self.issuer()]);

        let decoded = jsonwebtoken::decode(
            token,
            &DecodingKey::from_rsa_pem(public_key.as_ref())
                .context("Invalid public key format in ID token")?,
            &validation,
        )?;

        Ok(decoded.claims)
    }

    /// Reads an ID token of the Auth emulator: the claims are checked as for
    /// a real token (issued for this project, by its issuer, not expired, and
    /// naming a user), the signature is not, because the emulator writes none.
    fn decode_unsigned_id_token<C: DeserializeOwned>(
        &self,
        token: &str,
    ) -> Result<C, anyhow::Error> {
        #[derive(Deserialize)]
        struct RegisteredClaims {
            aud: Option<serde_json::Value>,
            iss: Option<String>,
            exp: Option<u64>,
            sub: Option<String>,
        }

        let payload = unverified_payload(token)?;
        let registered: RegisteredClaims = serde_json::from_slice(&payload)
            .context("The ID token's payload is not a set of JWT claims")?;

        let project_id = &self.service_account.project_id;

        anyhow::ensure!(
            registered.aud == Some(serde_json::Value::String(project_id.clone())),
            "The ID token was not issued for project '{project_id}'"
        );
        anyhow::ensure!(
            registered.iss.as_deref() == Some(self.issuer().as_str()),
            "The ID token was not issued by '{}'",
            self.issuer()
        );
        anyhow::ensure!(
            registered
                .exp
                .is_some_and(|exp| exp + EXPIRY_LEEWAY_SECONDS > get_current_timestamp()),
            "The ID token has expired"
        );
        anyhow::ensure!(
            registered.sub.is_some_and(|sub| !sub.is_empty()),
            "The ID token names no user"
        );

        serde_json::from_slice(&payload).context("Failed to read the ID token's claims")
    }

    /// Creates and signs a custom token for a user ID, which the user can use
    /// to authenticate against Firebase services.
    ///
    /// See the official [Firebase Auth docs for creating custom tokens](https://firebase.google.com/docs/auth/admin/create-custom-tokens#create_custom_tokens_using_a_third-party_jwt_library>).
    pub async fn create_custom_token(&self, uid: &str) -> Result<String, anyhow::Error> {
        #[derive(Serialize)]
        struct CustomTokenClaims<'a> {
            aud: &'a str,
            iat: u64,
            exp: u64,
            iss: &'a str,
            sub: &'a str,
            uid: &'a str,
        }

        let issued_at_time = get_current_timestamp();
        let expires_at = issued_at_time + (60 * 60);

        let claims = CustomTokenClaims {
            iss: &self.service_account.client_email,
            sub: &self.service_account.client_email,
            aud: FIREBASE_AUDIENCE,
            iat: issued_at_time,
            exp: expires_at,
            uid,
        };

        if self.emulator {
            // What the official Admin SDKs send the Auth emulator: the same
            // claims with `alg: none` and an empty signature.
            let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
            let claims = serde_json::to_vec(&claims)
                .context("Failed to serialize the custom token's claims")?;

            return Ok(format!("{header}.{}.", URL_SAFE_NO_PAD.encode(claims)));
        }

        let header = jsonwebtoken::Header::new(Algorithm::RS256);

        let encoding_key =
            jsonwebtoken::EncodingKey::from_rsa_pem(self.service_account.private_key.as_bytes())
                .context("Failed to create JWT encoding key from the given private key")?;

        let jwt = jsonwebtoken::encode(&header, &claims, &encoding_key)
            .context("Failed to create custom token JWT")?;

        Ok(jwt)
    }
}

/// The payload of a JWT as its bytes, with nothing verified: the token must
/// have its three parts, and the middle one must be base64url.
fn unverified_payload(token: &str) -> Result<Vec<u8>, anyhow::Error> {
    let mut parts = token.split('.');

    let (Some(header), Some(payload), Some(_signature), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        anyhow::bail!("The ID token is not a JWT: it does not have three parts");
    };

    anyhow::ensure!(
        !header.is_empty() && !payload.is_empty(),
        "The ID token is not a JWT: it has an empty part"
    );

    URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .context("The ID token's payload is not base64url")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const PROJECT: &str = "demo-fireplace";

    fn service_account() -> ServiceAccount {
        ServiceAccount {
            project_id: PROJECT.to_string(),
            private_key: String::new(),
            private_key_id: String::new(),
            client_email: "emulator@example.com".to_string(),
            client_id: "emulator".to_string(),
        }
    }

    fn emulator_manager() -> UserTokenManager {
        UserTokenManager::for_emulator(service_account(), reqwest::Client::new())
    }

    /// A token the way the Auth emulator writes one: `alg: none`, no signature.
    fn unsigned_token(claims: serde_json::Value) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
        format!("{header}.{payload}.")
    }

    fn valid_claims() -> serde_json::Value {
        json!({
            "aud": PROJECT,
            "iss": format!("https://securetoken.google.com/{PROJECT}"),
            "sub": "user-1",
            "user_id": "user-1",
            "iat": get_current_timestamp(),
            "exp": get_current_timestamp() + 3600,
            "email": "mario@example.com",
            "roles": ["campaignmanager"],
        })
    }

    #[derive(Deserialize, Debug)]
    struct Claims {
        sub: String,
        email: String,
        roles: Vec<String>,
    }

    #[tokio::test]
    async fn the_emulator_manager_reads_an_unsigned_id_token() {
        let claims: Claims = emulator_manager()
            .decode_id_token(&unsigned_token(valid_claims()))
            .await
            .unwrap();

        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.email, "mario@example.com");
        assert_eq!(claims.roles, ["campaignmanager"]);
    }

    #[tokio::test]
    async fn the_emulator_manager_still_checks_the_claims() {
        type Change = Box<dyn Fn(&mut serde_json::Value)>;

        let cases: Vec<(&str, Change, &str)> = vec![
            (
                "another project",
                Box::new(|c| c["aud"] = json!("demo-other")),
                "not issued for project",
            ),
            (
                "a list of audiences",
                Box::new(|c| c["aud"] = json!([PROJECT])),
                "not issued for project",
            ),
            (
                "no audience",
                Box::new(|c| {
                    c.as_object_mut().unwrap().remove("aud");
                }),
                "not issued for project",
            ),
            (
                "another issuer",
                Box::new(|c| c["iss"] = json!("https://securetoken.google.com/demo-other")),
                "not issued by",
            ),
            (
                "expired",
                Box::new(|c| c["exp"] = json!(get_current_timestamp() - 3600)),
                "expired",
            ),
            (
                "no expiry",
                Box::new(|c| {
                    c.as_object_mut().unwrap().remove("exp");
                }),
                "expired",
            ),
            (
                "no user",
                Box::new(|c| c["sub"] = json!("")),
                "names no user",
            ),
        ];

        for (name, change, expected) in cases {
            let mut claims = valid_claims();
            change(&mut claims);

            let error = emulator_manager()
                .decode_id_token::<Claims>(&unsigned_token(claims))
                .await
                .unwrap_err()
                .to_string();

            assert!(error.contains(expected), "{name}: {error}");
        }
    }

    #[tokio::test]
    async fn the_emulator_manager_refuses_what_is_not_a_jwt() {
        for token in ["", "abc", "a.b", "a.b.c.d", ".e30.", "e30..", "e30.***."] {
            let result = emulator_manager().decode_id_token::<Claims>(token).await;
            assert!(result.is_err(), "{token:?} was accepted");
        }
    }

    #[tokio::test]
    async fn a_real_manager_refuses_an_unsigned_id_token() {
        // The lock this whole mode rests on: without `for_emulator`, a token
        // with no signature never gets as far as its claims.
        let manager = UserTokenManager::new(service_account(), reqwest::Client::new());

        let result = manager
            .decode_id_token::<Claims>(&unsigned_token(valid_claims()))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn the_emulator_manager_writes_an_unsigned_custom_token() {
        let token = emulator_manager()
            .create_custom_token("user-1")
            .await
            .unwrap();

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[2], "", "an unsigned token has an empty signature");

        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        assert_eq!(header, json!({ "alg": "none", "typ": "JWT" }));

        let claims: serde_json::Value =
            serde_json::from_slice(&unverified_payload(&token).unwrap()).unwrap();
        assert_eq!(claims["uid"], "user-1");
        assert_eq!(claims["aud"], FIREBASE_AUDIENCE);
        assert!(claims["exp"].as_u64().unwrap() > get_current_timestamp());
    }
}
