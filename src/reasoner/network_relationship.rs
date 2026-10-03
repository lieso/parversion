use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use crate::basis_graph::NetworkRelationship;
use crate::basis_network::BasisNetwork;
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::graph_node::GraphNode;
use crate::prelude::*;
use crate::reasoner::{Capability, CompletionMetadata, Reasoner, ReasonerMetadata};
use crate::normal_context::NormalContext;
use crate::xslt::Xslt;

#[derive(Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NetworkRelationshipTypeResponse {
    ParentChild,
    NoRelationship,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct NetworkRelationshipSelfResponse {
    // Whether instances of this entity form a parent-child hierarchy ("PARENT_CHILD") or are independent of one another ("NO_RELATIONSHIP")
    pub relationship_type: NetworkRelationshipTypeResponse,
    // XSLT that, starting from a child instance's anchor element, locates its parent's anchor element, if PARENT_CHILD
    pub child_to_parent_xslt: Option<String>,
    // XSLT that, starting from a parent instance's anchor element, locates its direct children's anchor elements, if PARENT_CHILD
    pub parent_to_child_xslt: Option<String>,
}

pub async fn network_relationship<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>,
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    if left.id == right.id {
        network_relationship_reflexive(reasoner, Arc::clone(&normalization_context), left.clone())
            .await
    } else {
        network_relationship_comparative(
            reasoner,
            Arc::clone(&normalization_context),
            left.clone(),
            right.clone(),
        )
        .await
    }
}

async fn network_relationship_reflexive<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    network: Arc<BasisNetwork>,
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {

    let system_prompt = get_system_prompt_reflexive(reasoner, Arc::clone(&normalization_context)).await?;
    let user_prompt = get_user_prompt_reflexive(
        reasoner,
        Arc::clone(&normalization_context),
        network.clone(),
    )?;
    let capability = Capability::Capable;
    let schema = serde_json::to_value(schemars::schema_for!(NetworkRelationshipSelfResponse))
        .expect("Failed to serialise NetworkRelationshipSelfResponse schema");

    log::debug!("");
    log::debug!("╔═══════════════════════════════════════════════════════════════╗");
    log::debug!("║                                                               ║");
    log::debug!("║                   NETWORK RELATIONSHIP (SELF)                 ║");
    log::debug!("║                                                               ║");
    log::debug!("╚═══════════════════════════════════════════════════════════════╝");
    log::debug!("┌─── SYSTEM PROMPT ─────────────────────────────────────────────┐");
    log::debug!("{}", system_prompt);
    log::debug!("└───────────────────────────────────────────────────────────────┘");
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
    log::debug!("  Capability : {:?}", capability);
    log::debug!("");

    //let (result, metadata) = reasoner
    //    .execute::<NetworkRelationshipSelfResponse>(&capability, &system_prompt, &user_prompt, schema)
    //    .await?;

    //let reasoner_metadata = ReasonerMetadata {
    //    tokens: metadata.input_tokens + metadata.output_tokens,
    //    prompt_hash: metadata.prompt_hash.clone(),
    //};

    //log::debug!("result: {:?}", result);


    let parent = Arc::new(RwLock::new(GraphNode {
        id: ID::new(),
        parents: Vec::new(),
        description: "dummy_parent".to_string(),
        hash: Hash::new(),
        subgraph_hash: Hash::new(),
        lineage: Lineage::new(),
        children: Vec::new(),
    }));


    let normal_meta_context =
        network.apply(Arc::clone(&normalization_context), Arc::clone(&parent))?;

    let normal_contexts: Vec<Arc<NormalContext>> = {
        let root = read_lock!(normal_meta_context.graph_root);
        root.children
            .iter()
            .take(5)
            .map(|child| {
                let id = read_lock!(child).id.clone();
                normal_meta_context
                    .contexts_lookup
                    .get(&id)
                    .cloned()
                    .ok_or(Errors::DeficientNormalizationContextError(format!(
                        "No normal context found for graph node {}",
                        id.to_string()
                    )))
            })
            .collect::<Result<Vec<_>, Errors>>()?
    };


    let normal_context = normal_contexts.first().unwrap();


    let child_to_parent_xslt = r##"
    <xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
      <xsl:output method="xml" omit-xml-declaration="yes"/>

      <xsl:template match="a[contains(concat(' ', normalize-space(@class), ' '), ' hnuser ')]">
        <xsl:variable name="row"
          select="ancestor::tr[contains(concat(' ', normalize-space(@class), ' '), ' comtr ')][1]"/>
        <xsl:variable name="depth"
          select="number(($row//td[contains(concat(' ', normalize-space(@class), ' '), ' ind ')]/@indent)[1])"/>
        <xsl:copy-of select="
          $row/preceding-sibling::tr[contains(concat(' ', normalize-space(@class), ' '), ' comtr ')]
              [number((.//td[contains(concat(' ', normalize-space(@class), ' '), ' ind ')]/@indent)[1]) = $depth - 1]
              [1]
            //a[contains(concat(' ', normalize-space(@class), ' '), ' hnuser ')][1]"/>
      </xsl:template>
    </xsl:stylesheet>"##;

        let xslt = Xslt::new(&child_to_parent_xslt)?;

        log::info!("parsed xslt");



        for context in &normal_context.contexts {

            let result = xslt.traverse(
                Arc::clone(&normalization_context),
                context.graph_node.clone()
            )?;

            log::debug!("-----------------------------------------------------------------------------------------------------");
            log::debug!("result len: {}", result.len());

        }


//parent_to_child_xslt
//<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
//  <xsl:output method="xml" omit-xml-declaration="yes"/>
//
//  <xsl:template match="a[contains(concat(' ', normalize-space(@class), ' '), ' hnuser ')]">
//    <xsl:variable name="row"
//      select="ancestor::tr[contains(concat(' ', normalize-space(@class), ' '), ' comtr ')][1]"/>
//    <xsl:variable name="depth"
//      select="number(($row//td[contains(concat(' ', normalize-space(@class), ' '), ' ind ')]/@indent)[1])"/>
//    <xsl:copy-of select="
//      $row/following-sibling::tr[contains(concat(' ', normalize-space(@class), ' '), ' comtr ')]
//          [number((.//td[contains(concat(' ', normalize-space(@class), ' '), ' ind ')]/@indent)[1]) = $depth + 1]
//          [generate-id(
//             preceding-sibling::tr[contains(concat(' ', normalize-space(@class), ' '), ' comtr ')]
//               [number((.//td[contains(concat(' ', normalize-space(@class), ' '), ' ind ')]/@indent)[1]) = $depth][1]
//           ) = generate-id($row)]
//        //a[contains(concat(' ', normalize-space(@class), ' '), ' hnuser ')][1]"/>
//  </xsl:template>
//</xsl:stylesheet>

    unimplemented!()
}

async fn network_relationship_comparative<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    left: Arc<BasisNetwork>,
    right: Arc<BasisNetwork>,
) -> Result<(NetworkRelationship, ReasonerMetadata), Errors> {
    unimplemented!()
}

async fn get_system_prompt_reflexive<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
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

    let paths_to_try: Vec<String> = vec![
        format!(
            "{}/{}",
            document_type,
            meta_context.acyclic_subgraph_hash.clone()
        ),
        format!("{}", document_type),
    ];

    for path in paths_to_try {
        log::trace!("Searching for prompt with path: {}", path);
        if let Some(system_prompt) = reasoner.prompts().get(&path, "network_relationship_self").await? {
            return Ok(system_prompt);
        }
    }

    Err(Errors::UnavailableSystemPrompt(
        "Expected a network_relationship_self.txt system prompt in prompts directory".to_string(),
    ))
}

fn get_user_prompt_reflexive<R: Reasoner>(
    reasoner: &R,
    normalization_context: Arc<RwLock<NormalizationContext>>,
    network: Arc<BasisNetwork>,
) -> Result<String, Errors> {
    let parent = Arc::new(RwLock::new(GraphNode {
        id: ID::new(),
        parents: Vec::new(),
        description: "dummy_parent".to_string(),
        hash: Hash::new(),
        subgraph_hash: Hash::new(),
        lineage: Lineage::new(),
        children: Vec::new(),
    }));

    let normal_meta_context =
        network.apply(Arc::clone(&normalization_context), Arc::clone(&parent))?;

    let normal_contexts: Vec<Arc<NormalContext>> = {
        let root = read_lock!(normal_meta_context.graph_root);
        root.children
            .iter()
            .take(5)
            .map(|child| {
                let id = read_lock!(child).id.clone();
                normal_meta_context
                    .contexts_lookup
                    .get(&id)
                    .cloned()
                    .ok_or(Errors::DeficientNormalizationContextError(format!(
                        "No normal context found for graph node {}",
                        id.to_string()
                    )))
            })
            .collect::<Result<Vec<_>, Errors>>()?
    };

    let context_string = normal_contexts
        .iter()
        .try_fold(String::new(), |acc, normal_context| {
            let context_string = Context::generate_context_string_network_relationship(
                Arc::clone(&normalization_context),
                normal_context.contexts.clone()
            )?;

            Ok::<String, Errors>(if acc.is_empty() {
                context_string
            } else {
                format!("{}\n\n---SNIPPET SEPARATOR---\n\n{}", acc, context_string)
            })
        })?;

    let result = format!(r##"
[ENTITIES]
{}
    "##, context_string);

    Ok(result)
}
