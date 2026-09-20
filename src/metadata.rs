use crate::document::{DocumentRole, DocumentType};

#[derive(Clone, Debug)]
pub struct Metadata {
    pub document_type: Option<DocumentType>,
    pub origin: String,
    pub role: DocumentRole,
}
