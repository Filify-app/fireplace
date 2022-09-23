use crate::{
    error::FirebaseError,
    firestore::client::FirestoreClient,
    token::{FirebaseTokenProvider, ServiceAccount},
};

pub async fn initialise() -> Result<FirestoreClient, FirebaseError> {
    let service_account = ServiceAccount::from_file("./test-service-account.json").unwrap();
    let project_id = service_account.project_id().to_string();
    let token_provider = FirebaseTokenProvider::new(service_account);

    let client = FirestoreClient::initialise(&project_id, token_provider)
        .await
        .unwrap();

    Ok(client)
}
