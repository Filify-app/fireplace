use firestore_grpc::tonic;

#[derive(thiserror::Error)]
pub enum FirebaseError {
    #[error("{0}")]
    DocumentAlreadyExists(String),

    #[error("{0}")]
    DocumentNotfound(String),

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Failed to validate token: {0}")]
    ValidateTokenError(anyhow::Error),

    #[error(
        "serde: {source}{}",
        document.as_ref().map(|d| format!(" in document '{d}'")).unwrap_or_default())
    ]
    FirestoreSerdeError {
        source: crate::firestore::serde::Error,
        document: Option<String>,
    },

    #[error("grpc: {0}")]
    GrpcError(#[from] tonic::transport::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<crate::firestore::serde::Error> for FirebaseError {
    fn from(e: crate::firestore::serde::Error) -> Self {
        FirebaseError::FirestoreSerdeError {
            source: e,
            document: None,
        }
    }
}

impl std::fmt::Debug for FirebaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

// Taken from https://www.lpalmieri.com/posts/error-handling-rust/#internal-errors
fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
