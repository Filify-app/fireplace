use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    sync::Arc,
};

use anyhow::Context;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeStruct};

use super::query::{CollectionQuery, Filter};

pub fn collection(name: impl Into<String>) -> CollectionReference {
    CollectionReference::new(name)
}

/// A reference to a Firestore document.
#[derive(Debug, Clone)]
pub struct DocumentReference(Arc<DocumentReferenceInner>);

#[derive(Debug, Clone)]
pub struct CollectionReference(Arc<CollectionReferenceInner>);

#[derive(Debug, Clone)]
struct CollectionReferenceInner {
    parent: Option<DocumentReference>,
    name: String,
}

#[derive(Debug, Clone)]
struct DocumentReferenceInner {
    parent: CollectionReference,
    id: String,
}

static COLLECTION_REF_TYPE_ID: OnceCell<String> = OnceCell::new();

impl CollectionReference {
    pub fn new(collection_name: impl Into<String>) -> Self {
        Self(Arc::new(CollectionReferenceInner {
            parent: None,
            name: collection_name.into(),
        }))
    }

    pub fn doc(&self, id: impl Into<String>) -> DocumentReference {
        DocumentReference(Arc::new(DocumentReferenceInner {
            parent: self.clone(),
            id: id.into(),
        }))
    }

    pub fn parent(&self) -> Option<DocumentReference> {
        self.0.parent.clone()
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub(crate) fn type_id() -> &'static str {
        COLLECTION_REF_TYPE_ID.get_or_init(hashed_type_id::<Self>)
    }

    /// Create a Firestore query that filters documents from this collection.
    pub fn with_filter(self, filter: Filter<'_>) -> CollectionQuery<'_> {
        CollectionQuery::new(self).with_filter(filter)
    }

    /// Create a Firestore query that limits how many documents are returned.
    pub fn with_limit<'a>(self, limit: u32) -> CollectionQuery<'a> {
        CollectionQuery::new(self).with_limit(limit)
    }

    /// Create a Firestore query that specifies an offset for pagination.
    pub fn with_offset<'a>(self, offset: u32) -> CollectionQuery<'a> {
        CollectionQuery::new(self).with_offset(offset)
    }
}

impl Serialize for CollectionReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct(Self::type_id(), 1)?;
        s.serialize_field("relative_path", &self.to_string())?;
        s.end()
    }
}

impl TryFrom<String> for CollectionReference {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut slash_sep = value.split('/');
        let first = slash_sep.next().context("empty collection reference")?;
        let remaining = slash_sep.collect::<Vec<_>>();
        let mut parts = remaining.chunks_exact(2);

        let mut col_ref = collection(first);
        for part in parts.by_ref() {
            let (doc_id, collection_id) = (part[0], part[1]);
            col_ref = col_ref.doc(doc_id).collection(collection_id);
        }

        anyhow::ensure!(
            parts.remainder().is_empty(),
            "invalid amount of path segments in collection reference"
        );

        Ok(col_ref)
    }
}

impl<'de> Deserialize<'de> for CollectionReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

static DOC_REF_TYPE_ID: OnceCell<String> = OnceCell::new();

impl DocumentReference {
    pub fn collection(&self, name: impl Into<String>) -> CollectionReference {
        CollectionReference(Arc::new(CollectionReferenceInner {
            parent: Some(self.clone()),
            name: name.into(),
        }))
    }

    pub fn parent(&self) -> CollectionReference {
        self.0.parent.clone()
    }

    pub fn id(&self) -> &str {
        &self.0.id
    }

    pub(crate) fn type_id() -> &'static str {
        DOC_REF_TYPE_ID.get_or_init(hashed_type_id::<Self>)
    }
}

impl Serialize for DocumentReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct(Self::type_id(), 1)?;
        s.serialize_field("relative_path", &self.to_string())?;
        s.end()
    }
}

impl TryFrom<String> for DocumentReference {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let slash_sep = value.split('/').collect::<Vec<_>>();
        let mut parts = slash_sep.chunks_exact(2);

        let mut doc_ref = None;
        for part in parts.by_ref() {
            let (collection_id, doc_id) = (part[0], part[1]);
            doc_ref = match doc_ref {
                None => Some(collection(collection_id).doc(doc_id)),
                Some(parent) => Some(parent.collection(collection_id).doc(doc_id)),
            };
        }

        anyhow::ensure!(
            parts.remainder().is_empty(),
            "invalid amount of path segments in document reference"
        );

        doc_ref.context("empty document reference")
    }
}

impl<'de> Deserialize<'de> for DocumentReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Self::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl AsRef<Self> for DocumentReference {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<Self> for CollectionReference {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl std::fmt::Display for CollectionReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0.parent {
            Some(doc) => write!(f, "{}/{}", doc, self.0.name),
            None => write!(f, "{}", self.0.name),
        }
    }
}

impl std::fmt::Display for DocumentReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0.parent, self.0.id)
    }
}

impl PartialEq for CollectionReference {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl PartialEq for DocumentReference {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

fn hashed_type_id<T: 'static>() -> String {
    let type_id = TypeId::of::<T>();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    type_id.hash(&mut hasher);
    let hash = hasher.finish();
    hash.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_reference() {
        assert_eq!(CollectionReference::new("users").to_string(), "users");
    }

    #[test]
    fn document_reference() {
        assert_eq!(
            CollectionReference::new("users").doc("alice").to_string(),
            "users/alice"
        );
    }

    #[test]
    fn many_nested() {
        assert_eq!(
            CollectionReference::new("planets")
                .doc("tatooine")
                .collection("people")
                .doc("luke")
                .to_string(),
            "planets/tatooine/people/luke"
        );
    }

    #[test]
    fn deserialize_document_reference() {
        #[derive(Debug, Deserialize)]
        struct Test {
            doc_ref: DocumentReference,
        }

        let tatooine: Test = serde_json::from_str(r#"{"doc_ref": "planets/tatooine"}"#).unwrap();
        assert_eq!("planets/tatooine", tatooine.doc_ref.to_string());

        let luke: Test =
            serde_json::from_str(r#"{"doc_ref": "planets/tatooine/people/luke"}"#).unwrap();
        assert_eq!("planets/tatooine/people/luke", luke.doc_ref.to_string());
    }

    #[test]
    fn deserialize_invalid_document_reference_fails() {
        #[derive(Debug, Deserialize)]
        struct Test {
            #[allow(unused)]
            doc_ref: DocumentReference,
        }

        let res = serde_json::from_str::<Test>(r#"{"doc_ref": "planets"}"#);
        assert!(res.is_err(), "expected error, got {res:?}");
    }

    #[test]
    fn deserialize_collection_reference() {
        #[derive(Debug, Deserialize)]
        struct Test {
            col_ref: CollectionReference,
        }

        let test: Test = serde_json::from_str(r#"{"col_ref": "planets"}"#).unwrap();
        assert_eq!("planets", test.col_ref.to_string());
    }

    #[test]
    fn deserialize_invalid_collection_reference_fails() {
        #[derive(Debug, Deserialize)]
        struct Test {
            #[allow(unused)]
            col_ref: CollectionReference,
        }

        let res = serde_json::from_str::<Test>(r#"{"col_ref": "planets/tatooine"}"#);
        assert!(res.is_err(), "expected error, got {res:?}");
    }
}
