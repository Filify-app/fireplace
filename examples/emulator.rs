use fireplace::{
    ServiceAccount,
    auth::{FirebaseAuthClient, FirebaseAuthClientOptions, models::NewUser},
    firestore::{
        client::{FirestoreClient, FirestoreClientOptions},
        collection,
    },
};

#[tokio::main]
async fn main() {
    // The emulators check no credentials, so only the project ID matters. It
    // must start with `demo-`, the prefix Firebase reserves for projects that
    // exist only in the emulators: emulator mode is refused for any other.
    let service_account = ServiceAccount {
        project_id: "demo-fireplace".to_string(),
        private_key: String::new(),
        private_key_id: String::new(),
        client_email: "emulator@example.com".to_string(),
        client_id: "emulator".to_string(),
    };

    // This assumes that the local Firebase emulators are running, started
    // from this repository's firebase.json:
    //
    //   firebase emulators:start --only auth,firestore --project demo-fireplace
    //
    // which puts Firestore on port 8080 and Auth on port 9099.
    let firestore_options = FirestoreClientOptions::default().emulator_host("127.0.0.1:8080");
    let auth_options = FirebaseAuthClientOptions::default().emulator_host("127.0.0.1:9099");

    let mut firestore = FirestoreClient::initialise(service_account.clone(), firestore_options)
        .await
        .unwrap();
    let auth = FirebaseAuthClient::with_options(service_account, auth_options).unwrap();

    // Store a document.
    let doc_ref = collection("greetings").doc("first");
    let doc = serde_json::json!({ "message": "Hi Mom" });
    firestore.set_document(&doc_ref, &doc).await.unwrap();

    // Create a user, sign in as them and read the ID token back. The Auth
    // emulator's ID tokens are unsigned, and in emulator mode the client
    // accepts them on their claims.
    let user_id = auth
        .create_user(NewUser {
            display_name: Some("Mario".to_string()),
            email: format!("{}@example.com", ulid::Ulid::new()),
            password: ulid::Ulid::new().to_string(),
        })
        .await
        .unwrap();
    let custom_token = auth.create_custom_token(&user_id).await.unwrap();
    let id_token = auth.sign_in_with_custom_token(custom_token).await.unwrap();
    let claims: serde_json::Value = auth.decode_id_token(&id_token).await.unwrap();

    println!("Signed in as {}", claims["sub"]);
}
