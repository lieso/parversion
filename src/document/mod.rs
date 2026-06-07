use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};
use std::str::FromStr;

mod json;
mod xml;
mod html;

use crate::prelude::*;
use crate::document_format::DocumentFormat;
use crate::provider::Provider;
use crate::llm::LLM;

use json::Json;
use html::Html;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DocumentType {
    Json,
    PlainText,
    JavaScript,
    Xml,
    Html,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DocumentRole {
    Instance,
    Schema,
}

impl FromStr for DocumentRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "instance" => Ok(DocumentRole::Instance),
            "schema" => Ok(DocumentRole::Schema),
            other => Err(format!("Invalid document role: {}", other)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub origin: Option<String>,
    pub date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub document_type: DocumentType,
    pub data: String,
    pub metadata: DocumentMetadata,
}

impl Document {
    pub async fn from_schema_string<P: Provider>(
        provider: Arc<P>,
        value: String,
        options: &Options,
        metadata: &Metadata
    ) -> Result<Self, Errors> {
        log::trace!("In from_schema_string");

        if value.trim().is_empty() {
            return Err(Errors::DocumentNotProvided);
        }

        let mut hash = Hash::from_str(&value);
        hash.finalize();

        if !options.regenerate {
            if let Some(instance) = provider.get_instance_document_by_schema_hash(&hash).await? {
                return Ok(instance);
            }
        }

        let (instance, (tokens,)) = LLM::schema_to_instance(value).await?;

        let document = Document {
            document_type: metadata.document_type.clone().unwrap(),
            metadata: DocumentMetadata {
                origin: options.origin.clone(),
                date: options.date.clone(),
            },
            data: instance.clone(),
        };

        provider.save_schema_instance_document(
            &hash,
            document.clone(),
        ).await?;

        Ok(document)
    }

    pub fn from_string(
        value: String,
        options: &Options,
        metadata: &Metadata,
    ) -> Result<Self, Errors> {
        if value.trim().is_empty() {
            return Err(Errors::DocumentNotProvided);
        }

        let document = Document {
            document_type: metadata.document_type.clone().unwrap(),
            metadata: DocumentMetadata {
                origin: options.origin.clone(),
                date: options.date.clone(),
            },
            data: value,
        };

        Ok(document)
    }

    pub fn to_string(&self) -> String {
        self.data.clone()
    }

    pub fn generate_meta_context(&self) -> Result<MetaContext, Errors> {
        log::trace!("In generate_meta_context");

        match self.document_type {
            DocumentType::Json => Json::generate_meta_context(
                &self.metadata,
                self.data.clone()
            ),
            DocumentType::PlainText => unimplemented!(),
            DocumentType::JavaScript => unimplemented!(),
            DocumentType::Xml => unimplemented!(),
            DocumentType::Html => Html::generate_meta_context(
                &self.metadata,
                self.data.clone()
            ),
        }
    }

    pub fn from_normalized_graph(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        document_format: &DocumentFormat,
    ) -> Result<Self, Errors> {
        log::trace!("In from_normalized_graph");

        match document_format.format_type {
            DocumentType::Json => {
                let data = Json::from_normalized_graph(Arc::clone(&normalization_context))?;

                let document = Document {
                    document_type: DocumentType::Json,
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

    pub fn from_translated_graph(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        document_format: &DocumentFormat
    ) -> Result<Self, Errors> {
        log::trace!("In from_translated_graph");

        unimplemented!()
    }
}
