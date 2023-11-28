//! # Fireplace
//!
//! Fireplace is a client for Firebase that seeks to provide a user-friendly
//! interface to interact with Firestore, Firebase Auth, and similar.
//!
//! ## Firestore usage
//!
//! See the [`firestore`] module for more information.

pub mod auth;
pub mod error;
pub mod firestore;
mod service_account;

pub use service_account::ServiceAccount;
