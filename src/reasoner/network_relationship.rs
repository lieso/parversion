use std::sync::{Arc, RwLock};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::prelude::*;
use crate::reasoner::{Reasoner, ReasonerMetadata, Capability, CompletionMetadata};
use crate::basis_graph::NetworkRelationship;
use crate::transformation::RelationshipTransformation;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;

pub async fn network_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<NetworkRelationship>,
    right: Arc<NetworkRelationship>
) -> Result<(RelationshipTransformation, ReasonerMetadata), Errors> {
    log::trace!("In network_relationship");

    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        left.clone(),
        right.clone()
    ).await?;

    unimplemented!()
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<NetworkRelationship>,
    right: Arc<NetworkRelationship>,
) -> Result<String, Errors> {



    let left_normal_meta_context = left.apply(
        Arc::clone(&normalization_context),
    )?;


    let left_document = Document::from_normal_meta_context(
        &left_normal_meta_context,
        &DocumentFormat {
            format_type: DocumentType::Json,
            encoding: Some(String::from("UTF-8")),
            indent: None,
            line_ending: None,
            headers: None,
            wrap_text: None,
            exclude_nulls: None,
            custom_delimiter: None,
        },
    )?;

    log::debug!("left: {}", left_document.to_string());

    let right_normal_meta_context = right.apply(
        Arc::clone(&normalization_context),
    )?;


    let right_document = Document::from_normal_meta_context(
        &right_normal_meta_context,
        &DocumentFormat {
            format_type: DocumentType::Json,
            encoding: Some(String::from("UTF-8")),
            indent: None,
            line_ending: None,
            headers: None,
            wrap_text: None,
            exclude_nulls: None,
            custom_delimiter: None,
        },
    )?;

    log::debug!("right: {}", right_document.to_string());

    unimplemented!()
}
