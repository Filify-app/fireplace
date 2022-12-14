use fireplace::{
    firestore::{
        client::{FirestoreClient, FirestoreClientOptions},
        collection,
    },
    ServiceAccount,
};

#[tokio::main]
async fn main() {
    // Load the service account, which specifies which project we will connect
    // to and the secret keys used for authentication.
    let service_account = ServiceAccount::from_file("./test-service-account.json").unwrap();

    // Configure the client - we just want the default.
    let client_options = FirestoreClientOptions::default();

    // Finally, create a client for Firestore.
    let mut client = FirestoreClient::initialise(service_account, client_options)
        .await
        .unwrap();

    // Provide a document value and a reference to the location where we want
    // to store it.
    let doc_ref = collection("greetings").doc("first");
    let doc = serde_json::json!({ "message": "Hi Mom" });

    // Store it
    client.set_document(&doc_ref, &doc).await.unwrap();
}
