use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap};
use rand::prelude::*;

use crate::basis_network::BasisNetwork;
use crate::config::CONFIG;
use crate::network_relationship::NetworkRelationshipType;
use crate::prelude::*;
use crate::transformation::{FieldTransformation, FieldMetadata, NetworkTransformation, NetworkMetadata};
use crate::context::Context;

mod openai;
mod categorization;
mod node_analysis;
mod network_analysis;
mod network_relationships;

use node_analysis::{NodeAnalysis, LineageClassification};
use network_analysis::NetworkAnalysis;
use network_relationships::NetworkRelationships;

#[derive(Clone, Debug)]
pub enum NodeGroupClassification {
    Acyclic,
    Uniform,
    Diverging(Vec<Lineage>),
}

pub type NodeGroups = HashMap<Lineage, NodeGroupClassification>;

pub struct LLM {}

impl LLM {
    pub async fn get_parent_child_link(
        snippet: String,
    ) -> Result<((String, Vec<(String, String)>, Vec<(String, String)>, String), (u64,)), Errors> {
        log::trace!("In get_parent_child_link");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  PARENT CHILD LINK START                      ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let (response, metadata) = NetworkRelationships::get_parent_child_link(&snippet).await?;

        log::debug!("response: {:?}", response);

        let parent_value_xpaths = response.parent_value_xpaths
            .into_iter()
            .map(|v| (v.name, v.xpath))
            .collect();

        let candidate_value_xpaths = response.candidate_value_xpaths
            .into_iter()
            .map(|v| (v.name, v.xpath))
            .collect();

        Ok((
            (response.candidate_xpath, parent_value_xpaths, candidate_value_xpaths, response.filter_function),
            (metadata.tokens,),
        ))
    }

    pub async fn get_composition_link(
        snippet: String,
    ) -> Result<((String, String, String), (u64,)), Errors> {
        log::trace!("In get_composition_link");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  COMPOSITION LINK START                       ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let (response, metadata) = NetworkRelationships::get_composition_link(&snippet).await?;

        Ok(((response.forward_xpath, response.reverse_xpath, response.merged_variable_name), (metadata.tokens,)))
    }

    pub async fn identify_relationships(
        _meta_context: Arc<RwLock<MetaContext>>,
        original_document: String,
        network_jsons: Vec<(Arc<BasisNetwork>, Vec<String>)>
    ) -> Result<(Vec<(Arc<BasisNetwork>, Arc<BasisNetwork>, NetworkRelationshipType)>, (u64,)), Errors> {

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  IDENTIFY RELATIONSHIPS START                 ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let all_network_jsons: String = network_jsons.iter()
            .map(|(network, json_examples)| {
                let examples_string: String = json_examples.iter().enumerate()
                    .map(|(index, json)| format!("\nExample {}:\n{}\n", index + 1, json))
                    .collect();
                format!(
                    "\n{}\n\n[Network ID]\n{}\n\n[Network examples]\n{}\n",
                    "=".repeat(100),
                    network.id.to_string(),
                    examples_string
                )
            })
            .collect();

        let (relationships_response, metadata) = NetworkRelationships::identify_relationships(
            &original_document,
            &all_network_jsons,
        ).await?;

        log::debug!("relationships: {:?}", relationships_response.relationships);

        let relationships = relationships_response.relationships
            .into_iter()
            .map(|item| {
                let from = network_jsons.iter()
                    .find(|(n, _)| n.id.to_string() == item.from)
                    .map(|(n, _)| Arc::clone(n))
                    .unwrap_or_else(|| panic!("Relationship 'from' network not found: {}", item.from));
                let to = network_jsons.iter()
                    .find(|(n, _)| n.id.to_string() == item.to)
                    .map(|(n, _)| Arc::clone(n))
                    .unwrap_or_else(|| panic!("Relationship 'to' network not found: {}", item.to));
                let rel_type = serde_json::from_value(serde_json::Value::String(item.relationship_type.clone()))
                    .unwrap_or_else(|_| panic!("Unknown relationship type: {}", item.relationship_type));
                (from, to, rel_type)
            })
            .collect();

        Ok((relationships, (metadata.tokens,)))
    }

    pub async fn check_redundancy(
        _meta_context: Arc<RwLock<MetaContext>>,
        original_document: String,
        all_network_jsons: String
    ) -> Result<(Vec<String>, (u64,)), Errors> {

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  CHECK REDUNDANCY START                       ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let (redundancy_response, metadata) = NetworkRelationships::get_canonical_networks(
            &original_document,
            &all_network_jsons,
        ).await?;

        log::debug!("eliminated networks: {:?}", redundancy_response.eliminated);

        Ok((redundancy_response.canonical, (metadata.tokens,)))
    }

    pub async fn categorize(document: String) -> Result<
        (
            String, // name
            String, // description
            String, // structure
            Vec<String>, // aliases
            u64, // tokens
        ),
        Errors
    > {
        log::trace!("In categorize");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  CATEGORIZE GRAPH START                       ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let (categorization_response, metadata) = categorization::Categorization::categorize_graph(
            &document
        ).await?;

        let result = (
            categorization_response.category,
            categorization_response.description,
            categorization_response.structure,
            categorization_response.one_word_aliases
                .iter()
                .chain(
                    &categorization_response.two_word_aliases
                )
                .cloned()
                .collect(),
            metadata.tokens
        );

        Ok(result)
    }

    pub async fn get_network_transformation(
        subgraph_hash: &str,
        json_examples: &[String],
        document_summary: &str
    ) -> Result<(
        NetworkTransformation,
        u64 
    ), Errors> {
        log::trace!("In get_network_transformation");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  NETWORK TRANSFORMATION START                 ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");

        let result = NetworkAnalysis::get_network_transformation(
            json_examples,
            document_summary
        ).await?;

        let inference = result.data.unwrap();
        let meta = result.metadata;

        let network_transformation = NetworkTransformation {
            id: ID::new(),
            description: inference.description.clone(),
            subgraph_hash: subgraph_hash.to_string(),
            image: inference.name.to_string(),
            meta: NetworkMetadata {
                fields: inference.fields.clone(),
                cardinality: inference.cardinality.clone(),
                field_types: inference.field_types.clone(),
                context: inference.context.clone(),
                structure: inference.structure.clone(),
            }
        };

        Ok((network_transformation, (meta.tokens)))
    }

    pub async fn get_node_transformations(
        group: Vec<Arc<Context>>,
        meta_context: Arc<RwLock<MetaContext>>,
        document_summary: &str,
    ) -> Result<(
        Vec<FieldTransformation>,
        (u64,)
    ), Errors> {
        log::trace!("In get_node_transformations");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  NODE TRANSFORMATION START                    ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");

        let first = group.first().unwrap();
        let fields = first.data_node.fields.clone();

        let example_snippet_count = read_lock!(CONFIG).llm.example_snippet_count;
        let snippets: Vec<String> = group
            .iter()
            .take(example_snippet_count)
            .map(|c| c.generate_snippet(Arc::clone(&meta_context)))
            .collect();

        let mut field_transformations = Vec::new();
        let mut tokens: u64 = 0;

        for (field, value) in fields.into_iter() {
            let result = NodeAnalysis::get_node_transformation(
                &field,
                &value,
                snippets.clone(),
                document_summary,
            ).await?;

            if let Some(field_inference_response) = result.data {
                let transformation = FieldTransformation {
                    id: ID::new(),
                    description: field_inference_response.description,
                    field: field.to_string(),
                    image: field_inference_response.field_name,
                    meta: FieldMetadata {
                        data_type: field_inference_response.data_type,
                    }
                };

                field_transformations.push(transformation);
            }

            tokens += result.metadata.tokens;
        }

        Ok((field_transformations, (tokens,)))
    }

    pub async fn function_to_operation(code: &str) -> Result<Option<()>, Errors> {
        openai::OpenAI::function_to_operation(&code).await
    }

    pub async fn infer_group_match(
        meta_context: Arc<RwLock<MetaContext>>,
        group: Vec<Arc<Context>>,
    ) -> Result<(
        bool, // match
        (u64,)
    ), Errors> {
        log::trace!("In infer_group_match");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  INFER GROUP MATCH START                      ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");



        let mut seen = HashSet::new();

        for context in &group {
            if !seen.insert(context.id.clone()) {
                panic!("Why are there duplicate contexts here??");
            }
        }




        let sample_size = std::cmp::min(40, group.len());
        let use_all = group.len() <= 20;

        let sampled_contexts = if use_all {
            group.clone()
        } else {
            let mut rng = rand::rng();
            let mut shuffled = group.clone();
            shuffled.shuffle(&mut rng);
            shuffled.into_iter().take(sample_size).collect()
        };

        // TODO: a snippet points to an element, what if node has multiple attributes?
        let snippets: Vec<String> = sampled_contexts
            .iter()
            .map(|context: &Arc<Context>| context.generate_snippet(Arc::clone(&meta_context)))
            .collect();

        let (data, metadata) = crate::llm::node_analysis::NodeAnalysis::infer_snippets_match(snippets).await?;

        Ok((data.match_result, (metadata.tokens,)))
    }
}
