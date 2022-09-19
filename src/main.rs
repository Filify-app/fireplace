use firebase_admin_rs::firestore::{client::FirestoreClient, reference::CollectionReference};
use serde::Deserialize;

fn get_token() -> String {
    std::env::var("TOKEN").unwrap()
}

fn get_project_id() -> String {
    std::env::var("PROJECT_ID").unwrap()
}

#[tokio::main]
async fn main() {
    let project_id = get_project_id();
    let token = get_token();

    let mut client = FirestoreClient::initialise(&project_id, &token)
        .await
        .unwrap();

    let doc_ref = CollectionReference::new("greetings").doc("OGkyakVCxS7X419IGqvA");

    #[derive(Debug, Deserialize)]
    struct TestType {
        name: String,
    }

    let doc = client.get_document::<TestType>(&doc_ref).await;

    dbg!(doc);
}
