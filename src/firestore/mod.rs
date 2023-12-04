//! # Firestore
//!
//! - [Initializing the client](#initializing-the-client)
//! - [Query examples](#query-examples)
//!    * [Setting the documents](#setting-the-documents)
//!    * [Listing all documents in a collection](#listing-all-documents-in-a-collection)
//!    * [Filtering documents in a collection](#filtering-documents-in-a-collection)
//!    * [Collection group queries](#collection-group-queries)
//!    * [Using document metadata](#using-document-metadata)
//!    * [Paginated queries](#paginated-queries)
//!
//! ## Initializing the client
//!
//! ```no_run
//! # #[tokio::main]
//! # async fn main() {
//! use fireplace::{
//!     ServiceAccount,
//!     firestore::client::{FirestoreClient, FirestoreClientOptions}
//! };
//!
//! // Load the service account, which specifies which project we will connect
//! // to and the secret keys used for authentication.
//! let service_account = ServiceAccount::from_file("./test-service-account.json").unwrap();
//!
//! // Configure the client - we just want the default.
//! let client_options = FirestoreClientOptions::default();
//!
//! // Finally, create a client for Firestore.
//! let mut client = FirestoreClient::initialise(service_account, client_options)
//!     .await
//!     .unwrap();
//! # }
//! ```
//!
//! ## Query examples
//!
//! For the following examples, we will use the following database instance
//! that contains landmarks across some cities:
//!
//! ```text
//! cities (collection)
//! ├── SF (doc)
//! │   └── landmarks (collection)
//! │       ├── golden-gate: Golden Gate Bridge (type: bridge)
//! │       └── legion-honor: Legion of Honor (type: museum)
//! └── TOK (doc)
//!     └── landmarks (collection)
//!         └── national-science-museum: National Museum of Nature and Science (type: museum)
//! ```
//!
//! ### Setting the documents
//!
//! To write a document to Firestore, you can use the [`set_document`] method. The following
//! writes the documents specified above:
//!
//! [`set_document`]: crate::firestore::client::FirestoreClient::set_document
//!
//! ```
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use serde::Deserialize;
//! # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
//! use fireplace::firestore::collection;
//!
//! #[derive(Deserialize, Debug, PartialEq)]
//! struct Landmark {
//!     pub name: String,
//!     pub r#type: String,
//! }
//!
//! client
//!     .set_document(
//!         &collection("cities")
//!             .doc("SF")
//!             .collection("landmarks")
//!             .doc("golden-gate"),
//!         &serde_json::json!({ "name": "Golden Gate Bridge", "type": "bridge" }),
//!     )
//!     .await?;
//!
//! client
//!     .set_document(
//!         &collection("cities")
//!             .doc("SF")
//!             .collection("landmarks")
//!             .doc("legion-honor"),
//!         &serde_json::json!({ "name": "Legion of Honor", "type": "museum" }),
//!     )
//!     .await?;
//!
//! client
//!     .set_document(
//!         &collection("cities")
//!             .doc("TOK")
//!             .collection("landmarks")
//!             .doc("national-science-museum"),
//!         &serde_json::json!({ "name": "National Museum of Nature and Science", "type": "museum" }),
//!     )
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Listing all documents in a collection
//!
//! The following example lists all documents in `cities/SF/landmarks`:
//!
//! ```
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use fireplace::firestore::{collection, test_helpers::Landmark};
//! # let mut client = fireplace::firestore::test_helpers::initialise().await?;
//! # fireplace::firestore::test_helpers::setup_landmarks_example(&mut client).await?;
//! use futures::TryStreamExt;
//!
//! let query = collection("cities").doc("SF").collection("landmarks");
//! let sf_landmarks: Vec<Landmark> = client.run_query(query).await?.try_collect().await?;
//!
//! assert_eq!(
//!     sf_landmarks,
//!     vec![
//!         Landmark {
//!             name: "Golden Gate Bridge".to_string(),
//!             r#type: "bridge".to_string()
//!         },
//!         Landmark {
//!             name: "Legion of Honor".to_string(),
//!             r#type: "museum".to_string()
//!         },
//!     ]
//! );
//! # Ok(())
//! # }
//! ```
//!
//! ### Filtering documents in a collection
//!
//! The previous query can be extended with a [`filter`] to only return
//! documents that fulfil the given condition. For example, we can filter
//! the landmarks by their `type` field:
//!
//! [`filter`]: crate::firestore::query::filter
//!
//! ```
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use fireplace::firestore::{collection, test_helpers::Landmark};
//! # use futures::TryStreamExt;
//! # let mut client = fireplace::firestore::test_helpers::initialise().await?;
//! # fireplace::firestore::test_helpers::setup_landmarks_example(&mut client).await?;
//! use fireplace::firestore::query::{filter, EqualTo};
//!
//! let query = collection("cities")
//!     .doc("SF")
//!     .collection("landmarks")
//!     .with_filter(filter("type", EqualTo("museum")));
//! let sf_museums: Vec<Landmark> = client.run_query(query).await?.try_collect().await?;
//!
//! assert_eq!(
//!     sf_museums.into_iter().map(|m| m.name).collect::<Vec<_>>(),
//!     ["Legion of Honor"]
//! );
//! # Ok(())
//! # }
//! ```
//!
//! ### Collection group queries
//!
//! Collection group queries allow you to query across multiple collections
//! that share the same name. For example, we can query all museums across
//! all cities:
//!
//! ```
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use fireplace::firestore::{collection, test_helpers::Landmark, query::{filter, EqualTo}};
//! # use futures::TryStreamExt;
//! # let mut client = fireplace::firestore::test_helpers::initialise().await?;
//! # fireplace::firestore::test_helpers::setup_landmarks_example(&mut client).await?;
//! use fireplace::firestore::collection_group;
//!
//! let query = collection_group("landmarks").with_filter(filter("type", EqualTo("museum")));
//! let museums: Vec<Landmark> = client.run_query(query).await?.try_collect().await?;
//!
//! assert_eq!(
//!     museums.into_iter().map(|m| m.name).collect::<Vec<_>>(),
//!     ["Legion of Honor", "National Museum of Nature and Science"]
//! );
//! # Ok(())
//! # }
//! ```
//!
//! ### Using document metadata
//!
//! It is sometimes useful to obtain the document metadata, such as document
//! references to query results, or document creation and last-updated-at times.
//!
//! ```
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use fireplace::firestore::{collection, collection_group, test_helpers::Landmark, query::{filter, EqualTo}, client::FirestoreDocument};
//! # use futures::TryStreamExt;
//! # let mut client = fireplace::firestore::test_helpers::initialise().await?;
//! # fireplace::firestore::test_helpers::setup_landmarks_example(&mut client).await?;
//! let query = collection_group("landmarks").with_filter(filter("type", EqualTo("museum")));
//! let museums_with_metadata: Vec<FirestoreDocument<Landmark>> = client
//!     .run_query_with_metadata(query)
//!     .await?
//!     .try_collect()
//!     .await?;
//!
//! assert_eq!(
//!     museums_with_metadata
//!         .iter()
//!         .map(|m| &m.data.name)
//!         .collect::<Vec<_>>(),
//!     ["Legion of Honor", "National Museum of Nature and Science"]
//! );
//!
//! // For example, we can get document references to the retrieved documents
//!
//! let museum_references = museums_with_metadata
//!     .iter()
//!     .map(|m| m.document_reference())
//!     .collect::<Result<Vec<_>, _>>()?;
//!
//! assert_eq!(
//!     museum_references,
//!     [
//!         collection("cities")
//!             .doc("SF")
//!             .collection("landmarks")
//!             .doc("legion-honor"),
//!         collection("cities")
//!             .doc("TOK")
//!             .collection("landmarks")
//!             .doc("national-science-museum"),
//!     ]
//! );
//!
//! // Or we can get information about when the documents were created and last updated
//!
//! println!(
//!     "Document created at timestamp {:?} and last updated at {:?}",
//!     museums_with_metadata[0].create_time, museums_with_metadata[0].update_time
//! );
//! # Ok(())
//! # }
//! ```
//!
//! ### Paginated queries
//!
//! To paginate queries, you can specify limits and offsets.
//!
//! ```
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use fireplace::firestore::{collection_group, test_helpers::Landmark};
//! # use futures::TryStreamExt;
//! # let mut client = fireplace::firestore::test_helpers::initialise().await?;
//! # fireplace::firestore::test_helpers::setup_landmarks_example(&mut client).await?;
//! let query = collection_group("landmarks").with_limit(2);
//! let page_one: Vec<Landmark> = client.run_query(query).await?.try_collect().await?;
//!
//! let query = collection_group("landmarks").with_limit(2).with_offset(2);
//! let page_two: Vec<Landmark> = client.run_query(query).await?.try_collect().await?;
//!
//! assert_eq!(
//!     page_one.into_iter().map(|m| m.name).collect::<Vec<_>>(),
//!     ["Golden Gate Bridge", "Legion of Honor"]
//! );
//! assert_eq!(
//!     page_two.into_iter().map(|m| m.name).collect::<Vec<_>>(),
//!     ["National Museum of Nature and Science"]
//! );
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod query;
pub mod reference;
pub mod serde;
mod token_provider;

/// This module isn't really supposed to be exposed, but we are lacking
/// `#[cfg(doctest)]`, and we can't make it private either since doctests are
/// full-blown integration tests.
///
/// Relevant rust-lang issue: <https://github.com/rust-lang/rust/issues/67295>
pub mod test_helpers;

pub use query::collection_group;
pub use reference::collection;
