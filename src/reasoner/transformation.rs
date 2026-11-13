use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::reasoner::{Capability, Reasoner, ReasonerMetadata};
use crate::transformation::Transformation;

pub async fn transformation<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    field: String,
    image: String,
    description: String,
    contexts: Vec<Arc<Context>>,
) -> Result<(Transformation, ReasonerMetadata), Errors> {
    log::trace!("In transformation");

    let user_prompt = get_user_prompt(
        reasoner,
        Arc::clone(&normalization_context),
        &field,
        contexts.clone(),
    ).await?;

    log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
    log::debug!("{}", user_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");


    unimplemented!()
}

async fn get_user_prompt<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    field: &str,
    group: Vec<Arc<Context>>,
) -> Result<String, Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context
            .clone()
            .ok_or(Errors::DeficientNormalizationContextError(
                "Meta context not provided in normalization context".to_string(),
            ))?
    };

    let document_type = meta_context.document_type.to_string().to_lowercase();

    let origin = meta_context
        .document_metadata
        .origin
        .clone()
        .unwrap_or("Unknown".to_string());


    let context_strings: Vec<String> = group
        .iter()
        .take(3)
        .map(|context| {
            context.generate_context_string_basis_node(Arc::clone(&normalization_context))
        })
        .collect::<Result<Vec<String>, Errors>>()?;
    let snippets = context_strings.join("\n\n---SNIPPET SEPARATOR---\n\n");

    let values: String = group
        .iter()
        .flat_map(|context| {
            context.data_node.fields.get_all(field)
        })
        .fold(String::new(), |acc, field_value| {
            if acc.is_empty() {
                field_value.to_string()
            } else {
                format!("{}\n{}", acc, field_value.to_string())
            }
        });

    Ok(format!(r##"
[DOCUMENT TYPE]
{}

[SNIPPETS]
{}

[ORIGIN]
{}

[VALUES]
{}
"##, document_type, snippets, origin, values
    ))
}
