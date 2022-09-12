// use std::marker::PhantomData;
//
// #[derive(Debug, Clone)]
// pub struct Reference<T> {
//     path: Vec<String>,
//     phantom: PhantomData<T>,
// }
//
// #[derive(Debug, Clone)]
// pub struct Collection;
//
// #[derive(Debug, Clone)]
// pub struct Document;
//
// impl Reference<Collection> {
//     pub fn new(collection_name: String) -> Self {
//         Reference {
//             path: vec![collection_name],
//             phantom: PhantomData,
//         }
//     }
//
//     pub fn doc(&self, id: String) -> Reference<Document> {
//         let mut path = self.path.clone();
//         path.push(id);
//
//         Reference {
//             path,
//             phantom: PhantomData,
//         }
//     }
// }
//
// impl Reference<Document> {
//     pub fn collection(&self, collection_name: String) -> Reference<Document> {
//         let mut path = self.path.clone();
//         path.push(collection_name);
//
//         Reference {
//             path,
//             phantom: PhantomData,
//         }
//     }
// }

// #[derive(Debug, Clone)]
// pub struct CollectionReference {
//     parent: Option<Box<DocumentReference>>,
//     name: String,
// }
//
// #[derive(Debug, Clone)]
// pub struct DocumentReference {
//     parent: CollectionReference,
//     id: String,
// }
//
// impl CollectionReference {
//     pub fn new(collection_name: impl Into<String>) -> CollectionReference {
//         CollectionReference {
//             parent: None,
//             name: collection_name.into(),
//         }
//     }
//
//     pub fn doc(self, id: impl Into<String>) -> DocumentReference {
//         DocumentReference {
//             parent: self,
//             id: id.into(),
//         }
//     }
// }
//
// impl DocumentReference {
//     pub fn collection(self, name: impl Into<String>) -> CollectionReference {
//         CollectionReference {
//             parent: Some(Box::new(self)),
//             name: name.into(),
//         }
//     }
// }
//
// impl std::fmt::Display for CollectionReference {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match &self.parent {
//             Some(doc) => write!(f, "{}/{}", doc, self.name),
//             None => write!(f, "{}", self.name),
//         }
//     }
// }
//
// impl std::fmt::Display for DocumentReference {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}/{}", self.parent, self.id)
//     }
// }

// pub struct CollectionReference<'a> {
//     parent: Option<&'a DocumentReference<'a>>,
//     name: &'a str,
// }
//
// pub struct DocumentReference<'a> {
//     parent: &'a CollectionReference<'a>,
//     id: &'a str,
// }
//
// impl<'a> CollectionReference<'a> {
//     pub fn new(collection_name: &'a str) -> CollectionReference {
//         CollectionReference {
//             parent: None,
//             name: collection_name,
//         }
//     }
//
//     pub fn doc(&self, id: &'a str) -> DocumentReference {
//         DocumentReference { parent: self, id }
//     }
// }
//
// impl<'a> DocumentReference<'a> {
//     pub fn collection(&self, id: &'a str) -> CollectionReference {
//         CollectionReference {
//             parent: Some(self),
//             name: id,
//         }
//     }
// }
//
// impl std::fmt::Display for CollectionReference<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self.parent {
//             Some(doc) => write!(f, "{}/{}", doc, self.name),
//             None => write!(f, "{}", self.name),
//         }
//     }
// }
//
// impl std::fmt::Display for DocumentReference<'_> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}/{}", self.parent, self.id)
//     }
// }

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
}

impl DocumentReference {
    pub fn collection(&self, name: impl Into<String>) -> CollectionReference {
        CollectionReference(Rc::new(CollectionReferenceInner {
            parent: Some(self.clone()),
            name: name.into(),
        }))
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

// use std::borrow::Cow;
//
// #[derive(Debug, Clone)]
// pub struct CollectionReference<'a> {
//     parent: Option<Box<DocumentReference<'a>>>,
//     name: Cow<'a, str>,
// }
//
// #[derive(Debug, Clone)]
// pub struct DocumentReference<'a> {
//     parent: CollectionReference<'a>,
//     id: Cow<'a, str>,
// }
//
// impl<'a> CollectionReference<'a> {
//     pub fn new(collection_name: impl Into<Cow<'a, str>>) -> Self {
//         CollectionReference {
//             parent: None,
//             name: collection_name.into(),
//         }
//     }
//
//     pub fn doc(&self, id: impl Into<Cow<'a, str>>) -> DocumentReference {
//         DocumentReference {
//             parent: self.clone(),
//             id: id.into(),
//         }
//     }
// }
//
// impl<'a> DocumentReference<'a> {
//     pub fn collection(&self, name: impl Into<Cow<'a, str>>) -> CollectionReference {
//         CollectionReference {
//             parent: Some(Box::new(self.clone())),
//             name: name.into(),
//         }
//     }
// }
//
// impl<'a> std::fmt::Display for CollectionReference<'a> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match &self.parent {
//             Some(doc) => write!(f, "{}/{}", doc, self.name),
//             None => write!(f, "{}", self.name),
//         }
//     }
// }
//
// impl<'a> std::fmt::Display for DocumentReference<'a> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}/{}", self.parent, self.id)
//     }
// }
