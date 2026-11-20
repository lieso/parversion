use crate::document::{DocumentRole, DocumentType};

use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Metadata {
    pub document_type: Option<DocumentType>,
    pub url: Option<String>,
    pub date: Option<String>,
    pub role: DocumentRole,
    pub lineage: Option<Hash>,
}
