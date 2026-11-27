#[derive(Clone)]
pub struct FirestoreClientOptions {
    pub host_url: String,
    /// Whether `host_url` is a Firestore emulator. Set by
    /// [`emulator_host`](Self::emulator_host).
    pub emulator: bool,
}

impl Default for FirestoreClientOptions {
    fn default() -> Self {
        Self {
            host_url: "https://firestore.googleapis.com".to_string(),
            emulator: false,
        }
    }
}

impl FirestoreClientOptions {
    pub fn host_url(mut self, host_url: impl Into<String>) -> Self {
        self.host_url = host_url.into();
        self
    }

    /// Talk to the Firestore emulator at `host` (`host:port`, for example
    /// `127.0.0.1:8080`) instead of Google.
    ///
    /// Requests are then sent as the emulator's administrator (`Bearer
    /// owner`), so security rules are skipped as they are for a service
    /// account on the real service, and the service account's private key is
    /// never used: it may be empty.
    /// [`FirestoreClient::initialise`](super::FirestoreClient::initialise)
    /// refuses the mode for any project whose ID does not start with `demo-`,
    /// the prefix Firebase reserves for projects that exist only in the
    /// emulators.
    ///
    /// `localhost` is read as `127.0.0.1`: the emulator listens on IPv4.
    pub fn emulator_host(mut self, host: impl AsRef<str>) -> Self {
        self.host_url = format!("http://{}", crate::emulator::normalise_host(host.as_ref()));
        self.emulator = true;
        self
    }
}
