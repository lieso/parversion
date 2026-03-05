use crate::document::DocumentType;
use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct Metadata {
    pub document_type: Option<DocumentType>,
    pub origin: String,
}
