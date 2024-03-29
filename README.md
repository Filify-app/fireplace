# Fireplace

[![Crates.io page](https://img.shields.io/crates/v/fireplace.svg?style=flat-square)](https://crates.io/crates/fireplace)
[![docs.rs docs](https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square)](https://docs.rs/fireplace)

*The bestest, best Firebase library for Rust because there are no other libraries.*

This is a home-made client for Firebase's Admin SDK that seeks to provide a user friendly interface to interact with Firestore, Firebase Auth, and similar.

## Dependencies

To verify ID tokens for Firebase Auth, OpenSSL is required. For installation, see the [`openssl` crate's documentation](https://docs.rs/openssl/latest/openssl/index.html).

## Examples

Check out the `examples` directory or view the crate documentation on [docs.rs](https://docs.rs/fireplace), which includes many examples. Test-run the hello-world example with:

```
cargo run --example hello
```

This requires you to fetch your service account JSON file as described below.

## Setup

The easiest way is to get your service account JSON file that can be used to authorize requests to Firebase. This file will also decide which project on Firebase to send requests to.

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

For testing, the following environment variables need to be set so the tests can connect to the cloud:

- `FIREBASE_PROJECT_ID`
- `FIREBASE_CLIENT_ID`
- `FIREBASE_CLIENT_EMAIL`
- `FIREBASE_PRIVATE_KEY`
- `FIREBASE_PRIVATE_KEY_ID`

They should correspond to their values from the service account JSON file.

Additionally, some of the Firestore tests may need indices to be created. See the error messages for which.
