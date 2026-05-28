use crate::document::{DocumentType, DocumentRole};

#[derive(Clone, Debug)]
pub struct Metadata {
    pub document_type: Option<DocumentType>,
    pub origin: String,
    pub role: DocumentRole,
}
