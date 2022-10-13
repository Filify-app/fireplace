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

    // This assumes that a local Firebase emulator is running on port with a
    // config similar to this:
    // {
    //   "emulators": {
    //     "firestore": {
    //       "port": 8081
    //     },
    //     "auth": {
    //       "port": 9099
    //     },
    //     "ui": {
    //       "enabled": true
    //     }
    //   }
    // }
    // Important note: you must use 127.0.0.1 instead of localhost.
    let client_options = FirestoreClientOptions::default().host_url("https://127.0.0.1:8081");

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
