use std::fmt::Display;

use anyhow::{anyhow, Context};
use firestore_grpc::tonic;
use firestore_grpc::v1::firestore_client::FirestoreClient as GrpcFirestoreClient;
use firestore_grpc::v1::{CreateDocumentRequest, DocumentMask, UpdateDocumentRequest};
use firestore_grpc::{
    tonic::{
        codegen::InterceptedService,
        metadata::MetadataValue,
        transport::{Channel, ClientTlsConfig},
        Request, Status,
    },
    v1::GetDocumentRequest,
};
use serde::{Deserialize, Serialize};

use crate::error::FirebaseError;
use crate::firestore::serde::deserialize_firestore_document;

use super::reference::{CollectionReference, DocumentReference};
use super::serde::serialize_to_document;

type InterceptorFunction = Box<dyn Fn(Request<()>) -> Result<Request<()>, Status>>;

const URL: &str = "https://firestore.googleapis.com";
const DOMAIN: &str = "firestore.googleapis.com";

pub struct FirestoreClient {
    client: GrpcFirestoreClient<InterceptedService<Channel, InterceptorFunction>>,
    root_resource_path: String,
}

fn create_auth_interceptor(token: &str) -> InterceptorFunction {
    let bearer_token = format!("Bearer {}", token);
    let header_value = MetadataValue::from_str(&bearer_token).unwrap();

    Box::new(move |mut req: Request<()>| {
        req.metadata_mut()
            .insert("authorization", header_value.clone());
        Ok(req)
    })
}

impl FirestoreClient {
    pub async fn initialise(project_id: &str, token: &str) -> Result<Self, FirebaseError> {
        let endpoint =
            Channel::from_static(URL).tls_config(ClientTlsConfig::new().domain_name(DOMAIN));

        let channel = endpoint?.connect().await?;

        let service =
            GrpcFirestoreClient::with_interceptor(channel, create_auth_interceptor(token));

        let resource_path = format!("projects/{}/databases/(default)/documents", project_id);

        Ok(Self {
            client: service,
            root_resource_path: resource_path,
        })
    }

    /// Retrieve a document from Firestore at the given document reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use firebase_admin_rs::firestore::{client::FirestoreClient, reference::CollectionReference};
    /// # use serde::{Serialize, Deserialize};
    /// # let mut client = FirestoreClient::initialise(
    /// #     &std::env::var("PROJECT_ID").unwrap(),
    /// #     &std::env::var("TOKEN").unwrap(),
    /// # )
    /// # .await
    /// # .unwrap();
    /// #
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// struct Person {
    ///    name: String,
    /// }
    ///
    /// let collection_ref = CollectionReference::new("people");
    ///
    /// // First we create the document in the database
    /// let doc_id = client
    ///    .create_document(&collection_ref, None, &Person { name: "Luke Skywalker".to_string() })
    ///    .await
    ///    .unwrap();
    ///
    /// // Then we can retrieve it
    /// let doc_ref = collection_ref.doc(doc_id);
    /// let doc = client
    ///     .get_document(&doc_ref)
    ///     .await
    ///     .unwrap();
    ///
    /// assert_eq!(
    ///     doc,
    ///     Some(Person { name: "Luke Skywalker".to_string() })
    /// );
    ///
    /// // This document doesn't exist in the database, so we get a None.
    /// let doc_ref = CollectionReference::new("people").doc("luke-right-hand");
    /// let doc = client
    ///     .get_document::<Person>(&doc_ref)
    ///     .await
    ///     .unwrap();
    ///
    /// assert_eq!(doc, None);
    /// # }
    /// ```
    pub async fn get_document<'de, T: Deserialize<'de>>(
        &mut self,
        doc_ref: &DocumentReference,
    ) -> Result<Option<T>, FirebaseError> {
        let request = GetDocumentRequest {
            name: self.get_name_with(doc_ref),
            mask: None,
            consistency_selector: None,
        };

        let res = self.client.get_document(request).await;

        match res {
            Ok(res) => {
                let doc = res.into_inner();
                let deserialized = deserialize_firestore_document::<T>(doc)?;
                Ok(Some(deserialized))
            }
            Err(err) if err.code() == tonic::Code::NotFound => Ok(None),
            Err(err) => Err(anyhow!(err).into()),
        }
    }

    /// Creates a document in Firestore in the given collection. You can choose
    /// to provide a document ID, but Firestore will generate one for you if
    /// you don't. The ID of the created document will be returned.
    ///
    /// Returns an error if the document already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use firebase_admin_rs::error::FirebaseError;
    /// # use firebase_admin_rs::firestore::{client::FirestoreClient, reference::CollectionReference};
    /// # use serde::Serialize;
    /// # let mut client = FirestoreClient::initialise(
    /// #     &std::env::var("PROJECT_ID").unwrap(),
    /// #     &std::env::var("TOKEN").unwrap(),
    /// # )
    /// # .await
    /// # .unwrap();
    /// #
    /// #[derive(Debug, Serialize, PartialEq)]
    /// struct Greeting {
    ///     message: &'static str,
    /// }
    ///
    /// let collection_ref = CollectionReference::new("greetings");
    /// let doc_to_create = Greeting { message: "Hi Mom!" };
    ///
    /// // Create a document in the "greetings" collection, letting Firestore
    /// // generate the document's ID for us.
    /// let first_doc_id = client
    ///     .create_document(&collection_ref, None, &doc_to_create)
    ///     .await
    ///     .unwrap();
    ///
    /// // If we create another document with the same ID, it should fail
    /// let second_create_result = client
    ///     .create_document(&collection_ref, Some(first_doc_id), &doc_to_create)
    ///     .await;
    ///
    /// assert!(matches!(
    ///     second_create_result.unwrap_err(),
    ///     FirebaseError::DocumentAlreadyExists(_),
    /// ));
    /// # }
    /// ```
    pub async fn create_document<T: Serialize>(
        &mut self,
        collection_ref: &CollectionReference,
        document_id: Option<String>,
        document: &T,
    ) -> Result<String, FirebaseError> {
        // We should provide no name or timestamps when creating a document
        // according to Google's Firestore API reference.
        let doc = serialize_to_document(document, None, None, None)?;

        let request = CreateDocumentRequest {
            parent: self.root_resource_path.clone(),
            collection_id: collection_ref.to_string(),
            // Passing an empty string means that Firestore will generate a
            // document ID for us.
            document_id: document_id.unwrap_or_default(),
            document: Some(doc),
            mask: Some(DocumentMask {
                field_paths: vec![],
            }),
        };

        let res = self.client.create_document(request).await;

        match res {
            Ok(r) => {
                let created_doc = r.into_inner();
                let created_doc_id = created_doc
                    .name
                    .rsplit_once('/')
                    .map(|(_, id)| id.to_string())
                    .context("Could not get document ID from resource path")?;
                Ok(created_doc_id)
            }
            Err(err) if err.code() == tonic::Code::AlreadyExists => Err(
                FirebaseError::DocumentAlreadyExists(err.message().to_string()),
            ),
            Err(err) => Err(anyhow!(err).into()),
        }
    }

    /// Sets a document at the given document reference. If it doesn't already,
    /// exist, it is created - and if it does exist already, it is overwritten.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use firebase_admin_rs::firestore::{client::FirestoreClient, reference::{CollectionReference, DocumentReference}};
    /// # let mut client = FirestoreClient::initialise(
    /// #     &std::env::var("PROJECT_ID").unwrap(),
    /// #     &std::env::var("TOKEN").unwrap(),
    /// # )
    /// # .await
    /// # .unwrap();
    /// #
    /// let doc_ref = CollectionReference::new("greetings").doc("some-doc-id-to-set");
    /// let doc = serde_json::json!({ "message": "Hello, world!".to_string() });
    ///
    /// // We can upsert the document in the database
    /// client.set_document(&doc_ref, &doc).await.unwrap();
    ///
    /// // We can write to the same document reference again, and it will overwrite
    /// // the existing value document
    /// client.set_document(&doc_ref, &doc).await.unwrap();
    /// # }
    /// ```
    pub async fn set_document<T: Serialize>(
        &mut self,
        doc_ref: &DocumentReference,
        document: &T,
    ) -> Result<(), FirebaseError> {
        let name = self.get_name_with(doc_ref);
        let doc = serialize_to_document(document, Some(name), None, None)?;

        let request = UpdateDocumentRequest {
            document: Some(doc),
            update_mask: None,
            mask: Some(DocumentMask {
                field_paths: vec![],
            }),
            current_document: None,
        };

        self.client
            .update_document(request)
            .await
            .map_err(|err| anyhow!(err))?;

        Ok(())
    }

    /// Similar to [`set_document`](Self::set_document) but only upserts the
    /// fields specified in the `fields` argument.
    ///
    /// Generic type parameters: `I` for the input type that's to be serialized
    /// and `O` for the returned (full) document that should be deserialized.
    ///
    /// # Field selectors
    ///
    /// A simple field name contains only characters `a` to `z`, `A` to `Z`, `0`
    /// to `9`, or `_`, and must not start with `0` to `9`. For example,
    /// `foo_bar_17`.
    ///
    /// Field names matching the regular expression `__.*__` are reserved.
    /// Reserved field names are forbidden except in certain documented
    /// contexts. The map keys, represented as UTF-8, must not exceed 1,500
    /// bytes and cannot be empty.
    ///
    /// Field paths may be used in other contexts to refer to structured fields
    /// defined here. For map-like values, the field path is represented by the
    /// simple or quoted field names of the containing fields, delimited by `.`.
    /// For example, the field `"foo": { "x&y": "hello" }` would be represented
    /// by the field path `foo.x&y`.
    ///
    /// The above is a slightly modified description from the [Firestore API reference](https://firebase.google.com/docs/firestore/reference/rpc/google.firestore.v1#document).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use firebase_admin_rs::firestore::{client::FirestoreClient, reference::{CollectionReference, DocumentReference}};
    /// # use serde::{Deserialize, Serialize};
    /// # let mut client = FirestoreClient::initialise(
    /// #     &std::env::var("PROJECT_ID").unwrap(),
    /// #     &std::env::var("TOKEN").unwrap(),
    /// # )
    /// # .await
    /// # .unwrap();
    /// #
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// struct TestType {
    ///     label: String,
    ///     nested: NestedItem,
    /// }
    ///
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// #[serde(rename_all = "camelCase")]
    /// struct NestedItem {
    ///     field_a: String,
    ///     field_b: String,
    /// }
    ///
    /// // First, we set a document in the database
    /// let doc_ref = CollectionReference::new("greetings").doc("some-doc-id-to-set-merge");
    /// client
    ///     .set_document(
    ///         &doc_ref,
    ///         &TestType {
    ///             label: "Hello".to_string(),
    ///             nested: NestedItem {
    ///                 field_a: "A".to_string(),
    ///                 field_b: "B".to_string(),
    ///             },
    ///         },
    ///     )
    ///     .await
    ///     .unwrap();
    ///
    /// // Then we can update some fields of a document in the database. For
    /// // example, we can specify a top-level field ("label") or a nested field
    /// // ("nested.fieldA").
    /// let updated_doc: TestType = client
    ///     .set_document_merge(
    ///         &doc_ref,
    ///         &TestType {
    ///             label: "World".to_string(),
    ///             nested: NestedItem {
    ///                 field_a: "C".to_string(),
    ///                 field_b: "D".to_string(),
    ///             },
    ///         },
    ///         &["label", "nested.fieldB"],
    ///     )
    ///     .await
    ///     .unwrap();
    ///
    /// // Only the specified fields are updated. Despite `nested.field_a` having a
    /// // new value in the update, the value in the database is not changed.
    /// assert_eq!(
    ///     updated_doc,
    ///     TestType {
    ///         label: "World".to_string(),
    ///         nested: NestedItem {
    ///             field_a: "A".to_string(), // Notice this field did not change
    ///             field_b: "D".to_string(),
    ///         },
    ///     }
    /// );
    /// # }
    /// ```
    pub async fn set_document_merge<'de, I: Serialize, O: Deserialize<'de>>(
        &mut self,
        doc_ref: &DocumentReference,
        document: &I,
        // In reality we need a `Vec<String>`, but in by far most of the use-
        // cases, the user will be hard-coding the field names, so this makes
        // it much easier to just do that.
        fields: &[&str],
    ) -> Result<O, FirebaseError> {
        let name = self.get_name_with(doc_ref);
        let doc = serialize_to_document(document, Some(name), None, None)?;

        let request = UpdateDocumentRequest {
            document: Some(doc),
            update_mask: Some(DocumentMask {
                field_paths: fields.iter().map(|s| s.to_string()).collect(),
            }),
            mask: None,
            current_document: None,
        };

        let res = self
            .client
            .update_document(request)
            .await
            .map_err(|err| anyhow!(err))?;

        let doc = res.into_inner();
        let deserialized = deserialize_firestore_document::<O>(doc)?;

        Ok(deserialized)
    }

    fn get_name_with(&self, item: impl Display) -> String {
        format!("{}/{}", self.root_resource_path, item)
    }
}
