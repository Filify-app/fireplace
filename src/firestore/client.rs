use firestore_grpc::v1::firestore_client::FirestoreClient as GrpcFirestoreClient;
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

use crate::firestore::serde::deserialize_firestore_document;

use super::reference::DocumentReference;

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
    pub async fn initialise(project_id: &str, token: &str) -> Result<Self, anyhow::Error> {
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

    pub async fn get_document(&mut self, doc_ref: DocumentReference) {
        let name = format!("{}/{}", self.root_resource_path, doc_ref);

        let request = GetDocumentRequest {
            name,
            mask: None,
            consistency_selector: None,
        };

        let res = self.client.get_document(request).await;

        // TODO: convert this stuff to a bunch of tests
        #[derive(Debug, Serialize, Deserialize)]
        struct TestLulw {
            truthy: bool,
            message: String,
            idk: Option<String>,
            non_exist: Option<String>,
            mappy: MappyStuff,
            listy: Vec<MappyStuff>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct MappyStuff {
            hey: String,
        }

        let doc = res.unwrap().into_inner();

        dbg!(&doc);

        let res: TestLulw = deserialize_firestore_document(doc).unwrap();
        // TODO: do this as a test just to flex the generic stuff and type system
        // let res: serde_json::Value = from_grpc_document(doc).unwrap();

        dbg!(res);
    }
}
