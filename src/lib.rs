//! # Fireplace
//!
//! Fireplace is a Rust client library for Firebase's Admin SDK, providing ergouomic access to:
//!
//! - **Firestore**: Document database operations including CRUD, queries, and more
//! - **Firebase Auth**: User management, authentication, token verification, and more
//!
//! ## Firestore
//!
//! Firestore provides a NoSQL document database with querying capabilities. See the [`firestore`]
//! module for comprehensive examples and the [`FirestoreClient`] for the complete API reference.
//!
//! [`FirestoreClient`]: firestore::client::FirestoreClient
//!
//! ## Firebase Auth
//!
//! Firebase Auth provides user authentication and management capabilities.
//! See the [`auth`] module and [`FirebaseAuthClient`] for detailed documentation.
//!
//! [`FirebaseAuthClient`]: auth::FirebaseAuthClient

pub mod auth;
pub mod error;
pub mod firestore;
mod service_account;

pub use service_account::ServiceAccount;
