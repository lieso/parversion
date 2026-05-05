use std::sync::{Arc, RwLock};

mod json;

use crate::prelude::*;
use crate::json::Json;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DocumentType {
    Json,
    PlainText,
    JavaScript,
    Xml,
    Html,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub origin: Option<String>,
    pub date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub document_type: DocumentType,
    #[serde(skip_serializing)]
    pub data: String,
    pub metadata: DocumentMetadata,
}

impl Document {
    pub fn from_normalized_graph(
        meta_context: Arc<RwLock<MetaContext>>,
        document_format: &DocumentFormat,
    ) -> Result<Self, Errors> {
        log::trace!("In from_normalized_graph");

        match document_format.format_type {
            DocumentType::Json => {
                let data = Json::from_normalized_graph(Arc::clone(&meta_context))?;

                let document = Document {
                    document_type: DocumentJson::Json,
                    data,
                    metadata: DocumentMetadata {
                        origin: None,
                        date: None,
                    },
                };

                Ok(document)
            }
            DocumentType::PlainText => unimplemented!(),
            DocumentType::JavaScript => unimplemented!(),
            DocumentType::Xml => unimplemented!(),
            DocumentType::Html => unimplemented!(),
        }
    }
}
