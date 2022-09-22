use firebase_admin_rs::{
    firestore::{client::FirestoreClient, reference::CollectionReference},
    token::FirebaseTokenProvider,
};

#[tokio::main]
async fn main() {
    let token_provider =
        FirebaseTokenProvider::from_service_account_file("./test-service-account.json").unwrap();
    let project_id = token_provider.project_id().to_string();

    let mut client = FirestoreClient::initialise(&project_id, token_provider)
        .await
        .unwrap();

    let doc_ref = CollectionReference::new("greetings").doc("does-it-work?");
    let doc = serde_json::json!({ "message": "Yes indeed!".to_string() });

    client.set_document(&doc_ref, &doc).await.unwrap();
}
