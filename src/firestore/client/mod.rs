use std::fmt::Display;
use std::future;
use std::pin::Pin;

use anyhow::{anyhow, Context};
use firestore_grpc::tonic;
use firestore_grpc::v1::firestore_client::FirestoreClient as GrpcFirestoreClient;
use firestore_grpc::v1::run_query_request::QueryType;
use firestore_grpc::v1::structured_query::CollectionSelector;
use firestore_grpc::v1::{
    CreateDocumentRequest, DocumentMask, RunQueryRequest, StructuredQuery, UpdateDocumentRequest,
};
use firestore_grpc::{
    tonic::{
        codegen::InterceptedService, metadata::MetadataValue, transport::Channel, Request, Status,
    },
    v1::GetDocumentRequest,
};
use futures::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};

use crate::error::FirebaseError;
use crate::firestore::serde::deserialize_firestore_document;
use crate::ServiceAccount;

use super::query::Filter;
use super::reference::{CollectionReference, DocumentReference};
use super::serde::serialize_to_document;
use super::token_provider::FirestoreTokenProvider;

mod options;

pub use options::FirestoreClientOptions;

type FirebaseStream<T, E> = Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>;

type InterceptorFunction = Box<dyn FnMut(Request<()>) -> Result<Request<()>, Status> + Send>;

pub struct FirestoreClient {
    options: FirestoreClientOptions,
    client: GrpcFirestoreClient<InterceptedService<Channel, InterceptorFunction>>,
    grpc_channel: Channel,
    project_id: String,
    token_provider: FirestoreTokenProvider,
    root_resource_path: String,
}

impl Clone for FirestoreClient {
    fn clone(&self) -> Self {
        Self::from_channel(
            self.grpc_channel.clone(),
            self.token_provider.clone(),
            &self.project_id,
            self.options.clone(),
        )
    }
}

fn create_auth_interceptor(mut token_provider: FirestoreTokenProvider) -> InterceptorFunction {
    Box::new(move |mut req: Request<()>| {
        let token = token_provider
            .get_token()
            .map_err(|_| Status::unauthenticated("Could not get token from token provider"))?;

        let bearer_token = format!("Bearer {}", token);
        let mut header_value = MetadataValue::from_str(&bearer_token).map_err(|_| {
            Status::unauthenticated("Failed to construct metadata value for authorization token")
        })?;
        header_value.set_sensitive(true);

        req.metadata_mut().insert("authorization", header_value);

        Ok(req)
    })
}

impl FirestoreClient {
    /// Initialise a new client that can be used to interact with a Firestore
    /// database.
    pub async fn initialise(
        service_account: ServiceAccount,
        options: FirestoreClientOptions,
    ) -> Result<Self, FirebaseError> {
        let channel = Channel::from_shared(options.host_url.clone())
            .context("Failed to create gRPC channel")?
            .connect()
            .await
            .context("Failed to create channel to endpoint")?;

        let project_id = service_account.project_id.clone();
        let token_provider = FirestoreTokenProvider::new(service_account);

        Ok(Self::from_channel(
            channel,
            token_provider,
            &project_id,
            options,
        ))
    }

    fn from_channel(
        channel: Channel,
        token_provider: FirestoreTokenProvider,
        project_id: &str,
        options: FirestoreClientOptions,
    ) -> Self {
        // Cloning a channel is supposedly very cheap and encouraged be tonic's
        // documentation.
        let service = GrpcFirestoreClient::with_interceptor(
            channel.clone(),
            create_auth_interceptor(token_provider.clone()),
        );

        let resource_path = format!("projects/{}/databases/(default)/documents", project_id);

        Self {
            client: service,
            project_id: project_id.to_string(),
            token_provider,
            grpc_channel: channel,
            root_resource_path: resource_path,
            options,
        }
    }

    /// Retrieve a document from Firestore at the given document reference.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use serde::{Serialize, Deserialize};
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// #[derive(Debug, Serialize, Deserialize, PartialEq)]
    /// struct Person {
    ///    name: String,
    /// }
    ///
    /// let collection_ref = collection("people");
    ///
    /// // First we create the document in the database
    /// let doc_id = client
    ///    .create_document(&collection_ref, &Person { name: "Luke Skywalker".to_string() })
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
    /// let doc_ref = collection("people").doc("luke-right-hand");
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

    /// Creates a document in Firestore in the given collection, letting
    /// Firestore generate the ID for you. The ID of the created document will
    /// be returned.
    ///
    /// Returns an error if the document already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// let collection_ref = collection("greetings");
    /// let doc_to_create = serde_json::json!({ "message": "Hi Mom!" });
    ///
    /// let first_doc_id = client
    ///     .create_document(&collection_ref, &doc_to_create)
    ///     .await
    ///     .unwrap();
    ///
    /// println!("Created document with ID: {}", first_doc_id);
    /// # }
    /// ```
    pub async fn create_document<T: Serialize>(
        &mut self,
        collection_ref: &CollectionReference,
        document: &T,
    ) -> Result<String, FirebaseError> {
        self.create_document_internal(collection_ref, None, document)
            .await
    }

    /// Creates a document in Firestore at the given document reference.
    /// Returns the ID of the created document.
    ///
    /// Returns an error if the document already exists.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    /// # use fireplace::{firestore::collection, error::FirebaseError};
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// let collection_ref = collection("greetings");
    /// let doc_to_create = serde_json::json!({ "message": "Hi Mom!" });
    ///
    /// let first_doc_id = client
    ///     .create_document(&collection_ref, &doc_to_create)
    ///     .await
    ///     .unwrap();
    ///
    /// // If we create another document with the same ID, it should fail
    /// let second_create_result = client
    ///     .create_document_at_ref(&collection_ref.doc(first_doc_id), &doc_to_create)
    ///     .await;
    ///
    /// assert!(matches!(
    ///     second_create_result.unwrap_err(),
    ///     FirebaseError::DocumentAlreadyExists(_),
    /// ));
    /// # }
    /// ```
    pub async fn create_document_at_ref<T: Serialize>(
        &mut self,
        doc_ref: &DocumentReference,
        document: &T,
    ) -> Result<String, FirebaseError> {
        self.create_document_internal(&doc_ref.parent(), Some(doc_ref.id().to_string()), document)
            .await
    }

    async fn create_document_internal<T: Serialize>(
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
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
    /// #
    /// let doc_ref = collection("greetings").doc("some-doc-id-to-set");
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
    /// # use serde::{Deserialize, Serialize};
    /// # use fireplace::firestore::collection;
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await.unwrap();
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
    /// let doc_ref = collection("greetings").doc("some-doc-id-to-set-merge");
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

    /// Query a collection for documents that fulfill the given criteria.
    ///
    /// Returns a [`Stream`](futures::stream::Stream) of query results,
    /// allowing you to process results as they are coming in.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use fireplace::firestore::collection;
    /// # use serde::{Deserialize, Serialize};
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// #
    /// use fireplace::firestore::query::{filter, ArrayContains, EqualTo};
    /// use futures::TryStreamExt;
    ///
    /// #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    /// struct Pizza {
    ///     name: String,
    ///     toppings: Vec<String>,
    /// }
    ///
    /// // Instantiate our example pizzas
    /// let pepperoni = Pizza {
    ///     name: "Pepperoni".into(),
    ///     toppings: vec!["pepperoni".into(), "cheese".into()],
    /// };
    /// let hawaii = Pizza {
    ///     name: "Hawaii".into(),
    ///     toppings: vec!["pineapple".into(), "ham".into(), "cheese".into()],
    /// };
    ///
    /// // Create the pizzas in the database
    /// client
    ///     .set_document(&collection("pizzas").doc("pepperoni"), &pepperoni)
    ///     .await?;
    /// client
    ///     .set_document(&collection("pizzas").doc("hawaii"), &hawaii)
    ///     .await?;
    ///
    /// // Query for pizzas whose name field is "Hawaii"
    /// let hawaii_results: Vec<Pizza> = client
    ///     .query(&collection("pizzas"), filter("name", EqualTo("Hawaii"))?)
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// // We expect a single search hit - the hawaii pizza.
    /// assert_eq!(hawaii_results, vec![hawaii.clone()]);
    ///
    /// // Query for pizzas that have a "cheese" entry in the toppings list.
    /// let mut cheese_results: Vec<Pizza> = client
    ///     .query(
    ///         &collection("pizzas"),
    ///         filter("toppings", ArrayContains("cheese"))?,
    ///     )
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// // We don't have a guaranteed ordering of the query results, so we sort
    /// // them by name to make sure our equality check works.
    /// cheese_results.sort_by(|a, b| a.name.cmp(&b.name));
    ///
    /// // We expect both pizzas to be found
    /// assert_eq!(cheese_results, vec![hawaii, pepperoni]);
    ///
    /// // Query for pizzas with the name "pasta salad".
    /// let mut pasta_salad_results: Vec<Pizza> = client
    ///     .query(&collection("pizzas"), filter("name", EqualTo("pasta salad"))?)
    ///     .await?
    ///     .try_collect()
    ///     .await?;
    ///
    /// // We expect no results
    /// assert_eq!(pasta_salad_results, vec![]);
    /// # Ok(())
    /// # }
    pub async fn query<'de, T: Deserialize<'de>>(
        &mut self,
        collection: &CollectionReference,
        filter: Filter,
    ) -> Result<FirebaseStream<T, FirebaseError>, FirebaseError> {
        self.query_internal(collection, filter, None).await
    }

    /// The same as [`query`](Self::query), but only returns the first result.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use fireplace::firestore::collection;
    /// # use serde::{Deserialize, Serialize};
    /// # let mut client = fireplace::firestore::test_helpers::initialise().await?;
    /// #
    /// use fireplace::firestore::query::{filter, EqualTo};
    ///
    /// #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    /// struct Pizza {
    ///     name: String,
    /// }
    ///
    /// let hawaii = Pizza {
    ///     name: "Hawaii".into(),
    /// };
    ///
    /// client
    ///     .set_document(&collection("pizzas").doc("hawaii"), &hawaii)
    ///     .await?;
    ///
    /// // Query for the Hawaii pizza by name
    /// let mut hawaii_result: Option<Pizza> = client
    ///     .query_one(
    ///         &collection("pizzas"),
    ///         filter("name", EqualTo("Hawaii"))?,
    ///     )
    ///     .await?;
    ///
    /// // We expect a single search hit - the hawaii pizza.
    /// assert_eq!(hawaii_result, Some(hawaii.clone()));
    ///
    /// // Query for pizzas with the name "pasta salad".
    /// let mut pasta_salad_result: Option<Pizza> = client
    ///     .query_one(&collection("pizzas"), filter("name", EqualTo("pasta salad"))?)
    ///     .await?;
    ///
    /// // We expect no results
    /// assert_eq!(pasta_salad_result, None);
    /// # Ok(())
    /// # }
    pub async fn query_one<'de, T: Deserialize<'de>>(
        &mut self,
        collection: &CollectionReference,
        filter: Filter,
    ) -> Result<Option<T>, FirebaseError> {
        let mut stream = self.query_internal(collection, filter, Some(1)).await?;
        stream.try_next().await
    }

    async fn query_internal<'de, T: Deserialize<'de>>(
        &mut self,
        collection: &CollectionReference,
        filter: Filter,
        limit: Option<i32>,
    ) -> Result<FirebaseStream<T, FirebaseError>, FirebaseError> {
        let parent = collection
            .parent()
            .map(|p| self.get_name_with(p))
            .unwrap_or_else(|| self.root_resource_path.clone());

        let structured_query = StructuredQuery {
            select: None,
            from: vec![CollectionSelector {
                collection_id: collection.name().to_string(),
                // Setting all_descendants to false means we are only querying
                // the collection that is a direct child of the parent.
                all_descendants: false,
            }],
            r#where: Some(filter.into()),
            order_by: vec![],
            start_at: None,
            end_at: None,
            offset: 0,
            limit,
        };

        let request = RunQueryRequest {
            parent,
            query_type: Some(QueryType::StructuredQuery(structured_query)),
            consistency_selector: None,
        };

        let res = self
            .client
            .run_query(request)
            .await
            .context("Failed to run query")?;

        let doc_stream = res
            .into_inner()
            // Some of the "results" coming from the gRPC stream don't represent
            // search hits but rather information about query progress. We just
            // ignore those items.
            .filter(|res| match res {
                Ok(inner) => future::ready(inner.document.is_some()),
                Err(_) => future::ready(false),
            })
            .map(|res| {
                let doc = res
                    .context("Error response in query")?
                    .document
                    .context("No document in response - illegal state")?;

                let deserialized = deserialize_firestore_document::<T>(doc)?;

                Ok(deserialized)
            });

        Ok(doc_stream.boxed())
    }

    fn get_name_with(&self, item: impl Display) -> String {
        format!("{}/{}", self.root_resource_path, item)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn implements_send() {
        fn assert_send<T: Send>() {}
        assert_send::<super::FirestoreClient>();
    }
}
