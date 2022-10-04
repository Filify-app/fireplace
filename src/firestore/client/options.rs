#[derive(Clone)]
pub struct FirestoreClientOptions {
    pub host_url: String,
}

impl Default for FirestoreClientOptions {
    fn default() -> Self {
        Self {
            host_url: "https://firestore.googleapis.com".to_string(),
        }
    }
}

impl FirestoreClientOptions {
    pub fn host_url(mut self, host_url: impl Into<String>) -> Self {
        self.host_url = host_url.into();
        self
    }
}
