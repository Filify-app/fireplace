use anyhow::{anyhow, Context};
use firestore_grpc::tonic;
use firestore_grpc::v1::firestore_client::FirestoreClient as GrpcFirestoreClient;
use firestore_grpc::v1::CreateDocumentRequest;
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
        let name = format!("{}/{}", self.root_resource_path, doc_ref);

        let request = GetDocumentRequest {
            name,
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
    pub async fn create_document<'de, T: Serialize>(
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
            mask: None,
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
}
