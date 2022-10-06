use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Context;
use jsonwebtoken::{DecodingKey, Validation};
use serde::Deserialize;

pub(super) struct TokenHandler {
    public_keys: PublicKeys,
    project_id: String,
}

impl TokenHandler {
    pub(super) fn new(project_id: String, http_client: reqwest::Client) -> Self {
        Self {
            public_keys: PublicKeys::new(http_client),
            project_id,
        }
    }

    /// Verifies an ID token based on the docs at <https://firebase.google.com/docs/auth/admin/verify-id-tokens#verify_id_tokens_using_a_third-party_jwt_library>
    ///
    /// Fails if the token is in a bad format, expired, not issued for this
    /// project, or if the signature is invalid.
    pub(super) async fn decode_id_token(
        &mut self,
        token: &str,
    ) -> Result<IdTokenClaims, anyhow::Error> {
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
        validation.set_audience(&[&self.project_id]);
        validation.set_issuer(&[&format!(
            "https://securetoken.google.com/{}",
            &self.project_id
        )]);

        let decoded = jsonwebtoken::decode::<IdTokenClaims>(
            token,
            &DecodingKey::from_rsa_pem(public_key.as_ref())
                .context("Invalid public key format in ID token")?,
            &validation,
        )
        .context("Failed to decode ID token")?;

        Ok(decoded.claims)
    }
}

struct PublicKeys {
    public_key_map: Option<PublicKeyMap>,
    http_client: reqwest::Client,
}

impl PublicKeys {
    fn new(http_client: reqwest::Client) -> Self {
        Self {
            public_key_map: None,
            http_client,
        }
    }

    async fn get(&mut self, key_id: &str) -> Result<Option<&str>, anyhow::Error> {
        if self.should_update() {
            self.update().await?;
        }

        let key = self
            .public_key_map
            .as_ref()
            .context("Public key map was not present")?
            .keys
            .get(key_id)
            .map(String::as_str);

        Ok(key)
    }

    async fn update(&mut self) -> Result<(), anyhow::Error> {
        let public_key_map = PublicKeyMap::fetch(&self.http_client).await.map_err(|e| {
            tracing::error!("Failed to fetch public keys: {}", e);
            e
        })?;

        self.public_key_map = Some(public_key_map);

        Ok(())
    }

    fn should_update(&self) -> bool {
        match &self.public_key_map {
            None => true,
            Some(pkm) if Instant::now() >= pkm.update_by => true,
            _ => false,
        }
    }
}

struct PublicKeyMap {
    update_by: Instant,
    keys: HashMap<String, String>,
}

impl PublicKeyMap {
    const PUBLIC_KEYS_URL: &'static str =
        "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com";

    async fn fetch(client: &reqwest::Client) -> Result<Self, anyhow::Error> {
        tracing::debug!("Refreshing x509 public key certificates from Google");

        let res = client.get(Self::PUBLIC_KEYS_URL).send().await?;

        anyhow::ensure!(
            res.status().is_success(),
            "Google PKI returned status {}",
            res.status()
        );

        let headers = res.headers();

        let max_age = headers
            .get(reqwest::header::CACHE_CONTROL)
            .map(|h| h.to_str())
            .transpose()
            .context("Invalid Cache-Control header")?
            .and_then(|h| h.split(',').find(|s| s.trim().starts_with("max-age=")))
            .map(|s| {
                s.trim()
                    .trim_start_matches("max-age=")
                    .parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("Invalid max-age in Cache-Control header: {}", s))
            })
            .transpose()?
            .unwrap_or(5 * 60);

        let certificates = res.json::<HashMap<String, String>>().await?;
        let mut public_keys = HashMap::with_capacity(certificates.len());

        for (key_id, certificate_pem) in certificates {
            let certificate = openssl::x509::X509::from_pem(certificate_pem.as_bytes())?;
            let public_key_bytes = certificate.public_key()?.public_key_to_pem()?;
            let public_key = String::from_utf8(public_key_bytes)?;
            public_keys.insert(key_id, public_key);
        }

        Ok(Self {
            update_by: Instant::now() + Duration::from_secs(max_age),
            keys: public_keys,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct IdTokenClaims {
    pub user_id: String,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}
