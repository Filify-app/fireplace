use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::Context;
use tokio::sync::RwLock;

pub(super) struct PublicKeys {
    public_key_map: RwLock<Option<PublicKeyMap>>,
    http_client: reqwest::Client,
}

impl PublicKeys {
    pub fn new(http_client: reqwest::Client) -> Self {
        Self {
            public_key_map: RwLock::new(None),
            http_client,
        }
    }

    pub async fn get(&self, key_id: &str) -> Result<Option<String>, anyhow::Error> {
        if self.should_update().await {
            self.update().await?;
        }

        let public_key_map = self.public_key_map.read().await;

        let key = public_key_map
            .as_ref()
            .context("Public key map was not present")?
            .keys
            .get(key_id)
            .map(|s| s.to_owned());

        Ok(key)
    }

    async fn update(&self) -> Result<(), anyhow::Error> {
        let mut public_key_map = self.public_key_map.write().await;

        let pkm = PublicKeyMap::fetch(&self.http_client).await.map_err(|e| {
            tracing::error!("Failed to fetch public keys: {}", e);
            e
        })?;

        *public_key_map = Some(pkm);

        Ok(())
    }

    async fn should_update(&self) -> bool {
        match self.public_key_map.read().await.as_ref() {
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
