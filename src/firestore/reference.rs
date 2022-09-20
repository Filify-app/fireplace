use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct DocumentReference(Rc<DocumentReferenceInner>);

#[derive(Debug, Clone)]
pub struct CollectionReference(Rc<CollectionReferenceInner>);

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

impl CollectionReference {
    pub fn new(collection_name: impl Into<String>) -> Self {
        Self(Rc::new(CollectionReferenceInner {
            parent: None,
            name: collection_name.into(),
        }))
    }

    pub fn doc(&self, id: impl Into<String>) -> DocumentReference {
        DocumentReference(Rc::new(DocumentReferenceInner {
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
}

impl DocumentReference {
    pub fn collection(&self, name: impl Into<String>) -> CollectionReference {
        CollectionReference(Rc::new(CollectionReferenceInner {
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
}
