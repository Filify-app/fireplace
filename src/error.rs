use firestore_grpc::tonic;

#[derive(thiserror::Error)]
pub enum FirebaseError {
    #[error("{0}")]
    DocumentAlreadyExists(String),

    #[error(transparent)]
    FirestoreSerdeError(#[from] crate::firestore::serde::Error),

    #[error(transparent)]
    GrpcError(#[from] tonic::transport::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
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
