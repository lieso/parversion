use crate::prelude::*;
use crate::document::DocumentType;

#[derive(Clone, Debug)]
pub struct Metadata {
    pub document_type: Option<DocumentType>,
    pub origin: String,
}
