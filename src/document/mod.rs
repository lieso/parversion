use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap};

mod json;
mod xml;
mod html;

use crate::prelude::*;
use crate::provider::Provider;
use crate::profile::Profile;
use crate::context::Context;
use crate::graph_node::GraphNode;
use crate::document_format::DocumentFormat;
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

    pub async fn get_profile<P: Provider>(
        &self,
        provider: Arc<P>
    ) -> Result<Profile, Errors> {
        log::trace!("In get_profile");

        match self.document_type {
            DocumentType::Json => Json::get_profile(Arc::clone(&provider), self.data.clone()),
            DocumentType::PlainText => unimplemented!(),
            DocumentType::JavaScript => unimplemented!(),
            DocumentType::Xml => unimplemented!(),
            DocumentType::Html => Html::get_profile(Arc::clone(&provider), self.data.clone()),
        }
    }

    pub fn get_contexts(
        &self,
        meta_context: Arc<RwLock<MetaContext>>,
        metadata: &Metadata,
    ) -> Result<
        (
            HashMap<ID, Arc<Context>>, // context
            Arc<RwLock<GraphNode>>,    // graph root
        ),
        Errors,
    > {
        log::trace!("In get_contexts");

        match self.document_type {
            DocumentType::Json => unimplemented!(),
            DocumentType::PlainText => unimplemented!(),
            DocumentType::JavaScript => unimplemented!(),
            DocumentType::Xml => unimplemented!(),
            DocumentType::Html => Html::get_contexts(
                Arc::clone(&meta_context),
                metadata,
                self.data.clone()
            ),
        }
    }

    pub fn from_normalized_graph(
        meta_context: Arc<RwLock<MetaContext>>,
        document_format: &DocumentFormat,
    ) -> Result<Self, Errors> {
        log::trace!("In from_normalized_graph");

        match document_format.format_type {
            DocumentType::Json => {
                let data = Json::from_normalized_graph(Arc::clone(&meta_context))?;

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
}
