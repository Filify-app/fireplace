use firebase_admin_rs::{
    firestore::{client::FirestoreClient, reference::CollectionReference},
    token::FirebaseTokenProvider,
};

fn get_project_id() -> String {
    std::env::var("PROJECT_ID").unwrap()
}

#[tokio::main]
async fn main() {
    let project_id = get_project_id();

    let token_provider = FirebaseTokenProvider::from_service_account_file(
        "./local/rust-admin-sdk-test-firebase-adminsdk-g224e-8ecef5aee7.json",
    )
    .unwrap();

    let mut client = FirestoreClient::initialise(&project_id, token_provider)
        .await
        .unwrap();

    let doc_ref = CollectionReference::new("greetings").doc("does-it-work?");
    let doc = serde_json::json!({ "message": "Yes indeed!".to_string() });

    client.set_document(&doc_ref, &doc).await.unwrap();
}
