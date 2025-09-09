use fireplace::{
    ServiceAccount,
    auth::{FirebaseAuthClient, models::NewUser},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the service account, which specifies which project we will connect
    // to and the secret keys used for authentication.
    let service_account = ServiceAccount::from_file("./test-service-account.json").unwrap();

    // Create the auth client
    let auth_client = FirebaseAuthClient::new(service_account)?;

    // Create a new user
    let user_id = auth_client
        .create_user(NewUser {
            display_name: Some("Julius Caesar".to_string()),
            email: "caesar@rome.it".to_string(),
            password: "venividivici".to_string(),
        })
        .await?;

    // Get the user
    let user = auth_client.get_user(&user_id).await?;

    // See the user's Firebase Auth attributes
    dbg!(user);

    Ok(())
}
