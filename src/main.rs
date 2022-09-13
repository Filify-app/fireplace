use firebase_admin_rs::firestore::{client::FirestoreClient, reference::CollectionReference};

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

    client.get_document(doc_ref).await;
}
