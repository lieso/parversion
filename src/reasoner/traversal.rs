use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};

use super::sampling::sample_random;
use crate::prelude::*;
use crate::basis_node::BasisNode;
use crate::graph_node::Graph;
use crate::traversal::{Traversal, TraversalKind};
use crate::reasoner::{get_system_prompt, Capability, PromptName, Reasoner, ReasonerMetadata, ReasonerErrors};

#[derive(Deserialize, Serialize, JsonSchema, Debug)]
pub struct TraversalResponse {
    // The XPath to get from LEFT to RIGHT
    pub left_to_right_xpath: String,
    // The XPath to get from RIGHT to LEFT
    pub right_to_left_xpath: String,
}

pub async fn traversal<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
) -> Result<(Traversal, ReasonerMetadata), Errors> {
    log::trace!("In traversal");

    let feedback_loop_size = {
        let lock = read_lock!(normalization_context);
        lock.feedback_loop_size
    };

    let mut reasoner_metadata = ReasonerMetadata {
        tokens: 0,
        prompts: Vec::new(),
        cost: 0.0,
    };
    let mut previous_feedback: Option<String> = None;
    let mut previous_traversal: Option<Traversal> = None;

    for _ in 0..feedback_loop_size {
        let (next_traversal, next_reasoner_metadata) = determine_traversal(
            reasoner,
            Arc::clone(&normalization_context),
            Arc::clone(&left),
            Arc::clone(&right),
            Capability::XPath,
            previous_feedback,
        ).await?;

        reasoner_metadata = ReasonerMetadata {
            tokens: reasoner_metadata.tokens + next_reasoner_metadata.tokens,
            prompts: [reasoner_metadata.prompts, next_reasoner_metadata.prompts].concat(),
            cost: reasoner_metadata.cost + next_reasoner_metadata.cost,
        };

        if let Some(previous_traversal) = previous_traversal {
            if next_traversal == previous_traversal {
                return Ok((next_traversal, reasoner_metadata));
            }
        }

        let next_feedback = generate_feedback(
            Arc::clone(&normalization_context),
            Arc::clone(&left),
            Arc::clone(&right),
            &next_traversal,
        )?;

        previous_traversal = Some(next_traversal);
        previous_feedback = Some(next_feedback);
    }

    Err(ReasonerErrors::Unreasonable("Could not reason a traversal".to_string()).into())
}

async fn determine_traversal<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
    capability: Capability,
    feedback: Option<String>
) -> Result<(Traversal, ReasonerMetadata), Errors> {
    let meta_context = {
        let lock = read_lock!(normalization_context);
        lock.meta_context.clone().unwrap()
    };
    let document_lineage = read_lock!(meta_context.graph_root).lineage.clone();
    let system_prompt = get_system_prompt(
        reasoner,
        document_lineage,
        PromptName::Traversal,
    )
    .await?;
    let basis_node_contexts = {
        let lock = read_lock!(normalization_context);
        lock.basis_node_contexts.clone().ok_or_else(|| {
            Errors::DeficientNormalizationContextError(
                "Basis node contexts not provided in meta context".to_string(),
            )
        })?
    };
    let left_contexts: Vec<Arc<Context>> = basis_node_contexts.get(&left.id).unwrap().to_vec();
    let right_contexts: Vec<Arc<Context>> = basis_node_contexts.get(&right.id).unwrap().to_vec();

    let user_prompt = get_user_prompt(
        Arc::clone(&normalization_context),
        left.clone(),
        &left_contexts,
        right.clone(),
        &right_contexts,
        feedback,
    )
    .await?;
    let schema = serde_json::to_value(schemars::schema_for!(TraversalResponse))
        .expect("Failed to serialise TraversalResponse schema");

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                      TRAVERSAL                                ║");
    log::debug!("║                                                               ║");
    log::debug!("╚═══════════════════════════════════════════════════════════════╝");
    log::debug!("");
    log::debug!("  Capability : {:?}", capability);
    log::debug!("");
    log::debug!("┌─── SYSTEM PROMPT ─────────────────────────────────────────────┐");
    log::debug!("{}", system_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");
    log::debug!("┌─── USER PROMPT ───────────────────────────────────────────────┐");
    log::debug!("{}", user_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");
    log::debug!("┌─── SCHEMA ────────────────────────────────────────────────────┐");
    log::debug!(
        "{}",
        serde_json::to_string_pretty(&schema).unwrap_or_default()
    );
    log::debug!("└───────────────────────────────────────────────────────────────┘");
    log::debug!("");

    let (result, metadata) = reasoner
        .execute::<TraversalResponse>(&capability, &system_prompt, &user_prompt, schema)
        .await?;

    let reasoner_metadata = ReasonerMetadata {
        tokens: metadata.input_tokens + metadata.output_tokens,
        prompts: vec![metadata.prompt_hash.clone()],
        cost: metadata.cost.unwrap_or(0.0),
    };

    let traversal = Traversal {
        id: ID::new(),
        left_basis_lineage: Some(left.lineage.clone()),
        right_basis_lineage: Some(right.lineage.clone()),
        kind: TraversalKind::XPath {
            xpath_ltr: result.left_to_right_xpath.clone(),
            xpath_rtl: result.right_to_left_xpath.clone(),
        }
    };

    Ok((traversal, reasoner_metadata))
}

fn generate_feedback(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    right: Arc<BasisNode>,
    traversal: &Traversal,
) -> Result<String, Errors> {
    log::trace!("In generate_feedback");
    unimplemented!()
}

async fn get_user_prompt(
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNode>,
    left_contexts: &[Arc<Context>],
    right: Arc<BasisNode>,
    right_contexts: &[Arc<Context>],
    feedback: Option<String>
) -> Result<String, Errors> {
    fn make_context_string(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        basis_node: Arc<BasisNode>,
        contexts: &[Arc<Context>],
        other_contexts: &[Arc<Context>],
    ) -> Result<String, Errors> {
        let samples = sample_random(
            contexts.to_vec(),
            read_lock!(normalization_context)
                .context_sample_size
                .clone(),
        );

        samples.iter().try_fold(String::new(), |acc, context| {
            let context_string = context.generate_context_string_traversal(
                Arc::clone(&normalization_context),
                Arc::clone(&basis_node),
                other_contexts,
            )?;

            if acc.is_empty() {
                Ok(context_string)
            } else {
                Ok(format!("{}\n\n---SNIPPET SEPARATOR---\n\n{}", acc, context_string))
            }
        })
    }

    let left_context_string = make_context_string(
        Arc::clone(&normalization_context),
        Arc::clone(&left),
        left_contexts,
        right_contexts,
    )?;
    let right_context_string = make_context_string(
        Arc::clone(&normalization_context),
        Arc::clone(&left),
        right_contexts,
        left_contexts,
    )?;

    let mut result = format!(
        r##"
[LEFT]
{}

[RIGHT]
{}

"##,
        left_context_string, right_context_string
    );

    if let Some(feedback) = feedback {
        result.push_str(&format!("[FEEDBACK]\n{}", feedback));
    }

    Ok(result)
}
