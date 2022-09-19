use anyhow::anyhow;
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
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use firebase_admin_rs::firestore::{client::FirestoreClient, reference::CollectionReference};
    /// # use serde::Deserialize;
    /// # let mut client = FirestoreClient::initialise(
    /// #     &std::env::var("PROJECT_ID").unwrap(),
    /// #     &std::env::var("TOKEN").unwrap(),
    /// # )
    /// # .await
    /// # .unwrap();
    /// #
    /// #[derive(Debug, Deserialize, PartialEq)]
    /// struct Person {
    ///    name: String,
    /// }
    ///
    /// // This document exists in the database.
    /// let doc_ref = CollectionReference::new("people").doc("luke");
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
    /// // This document doesn't exist in the database.
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

    pub async fn create_document<'de, T: Serialize>(
        &mut self,
        collection_ref: &CollectionReference,
        document_id: Option<String>,
        document: &T,
    ) -> Result<(), FirebaseError> {
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
            Ok(_) => Ok(()),
            Err(err) if err.code() == tonic::Code::AlreadyExists => Err(
                FirebaseError::DocumentAlreadyExists(err.message().to_string()),
            ),
            Err(err) => Err(anyhow!(err).into()),
        }
    }
}
