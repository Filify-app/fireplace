use firebase_admin_rs::{
    firestore::{client::FirestoreClient, collection},
    token::{FirebaseTokenProvider, ServiceAccount},
};

#[tokio::main]
async fn main() {
    let service_account = ServiceAccount::from_file("./test-service-account.json").unwrap();
    let project_id = service_account.project_id().to_string();
    let token_provider = FirebaseTokenProvider::new(service_account);

    let mut client = FirestoreClient::initialise(&project_id, token_provider)
        .await
        .unwrap();

    let doc_ref = collection("greetings").doc("does-it-work?");
    let doc = serde_json::json!({ "message": "Yes indeed!".to_string() });

    client.set_document(&doc_ref, &doc).await.unwrap();
}
