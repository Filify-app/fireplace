# Fireblaze

This is a home-made client for Firebase's Admin SDK that seeks to provide a user friendly interface to interact with Firestore, Firebase Auth, and similar.

## Setup

You need to get your service account JSON file that will be used to authorize requests to Firebase. This file will also decide which project on Firebase to send requests to.

Your JSON file will look something like this:

```json
{
  "type": "service_account",
  "project_id": "...",
  "private_key_id": "...",
  "private_key": "-----BEGIN PRIVATE KEY----- ...",
  "client_email": "...",
  "client_id": "...",
  "auth_uri": "...",
  "token_uri": "...",
  "auth_provider_x509_cert_url": "...",
  "client_x509_cert_url": "..."
}
```

## Testing

Currently I've made the tests use a real in-the-cloud Firebase project to ensure that everything works as expected. However, this has the consequence that you need to set up access and be careful about tests affecting each other.

These tests require a `test-service-account.json` to be present in the project root that can provide access to that project.

