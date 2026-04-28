use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::basis_network::BasisNetwork;
use crate::config::CONFIG;
use crate::network_relationship::NetworkRelationshipType;
use crate::path::Path;
use crate::prelude::*;
use crate::schema_context::SchemaContext;
use crate::transformation::{FieldTransformation, SchemaTransformation, FieldMetadata, NetworkTransformation, NetworkMetadata};
use crate::context::Context;

mod openai;
mod translation;
mod categorization;
mod node_analysis;
mod network_analysis;
mod network_relationships;

use node_analysis::{NodeAnalysis, NodeGroupsData, LineageClassification};
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
        meta_context: Arc<RwLock<MetaContext>>,
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
        meta_context: Arc<RwLock<MetaContext>>,
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

    pub async fn translate_schema_node(
        meta_context: Arc<RwLock<MetaContext>>,
        schema_context:SchemaContext,
        target_schema: Arc<String>,
    ) -> Result<Option<(Path, Path)>, Errors> {
        log::trace!("In translate_schema_node");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                  TRANSLATE SCHEMA NODE START                  ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");
        log::debug!("┌─── SCHEMA NODE ───────────────────────────────────────────────┐");
        log::debug!("{}", schema_context.schema_node.to_string());
        log::debug!("└───────────────────────────────────────────────────────────────┘");
        log::debug!("");

        let snippet = schema_context.generate_snippet(Arc::clone(&meta_context));

        let match_response =
            translation::Translation::match_target_schema(&snippet, &target_schema).await?;

        log::debug!("match_response: {:?}", match_response);

        if match_response.is_incompatible || match_response.json_path.is_none() {
            log::debug!("Schema node is incompatible with target schema");
            return Ok(None);
        }

        let json_path = match_response.json_path.unwrap();

        let translation_schema = {
            let lock = read_lock!(meta_context);
            lock.translation_schema.clone().unwrap()
        };

        let maybe_schema_node = translation_schema.get_schema_node_by_json_path(&json_path);

        if let Some(target_schema_node) = maybe_schema_node {
            log::info!("Found target schema node");

            let (schema_node_path, target_node_path) = {
                let lock = read_lock!(meta_context);

                let schema_contexts = lock.schema_contexts.clone().unwrap();
                let schema_node_path: Path = schema_context.to_path(schema_contexts)?;

                let target_schema_context = lock
                    .translation_schema_contexts
                    .as_ref()
                    .unwrap()
                    .values()
                    .find(|sc| sc.schema_node.id == target_schema_node.id)
                    .unwrap();

                let target_schema_contexts = lock.translation_schema_contexts.clone().unwrap();
                let target_node_path: Path =
                    target_schema_context.to_path(target_schema_contexts)?;
                let target_node_path = target_node_path.with_unique_variables(&schema_node_path);

                (schema_node_path, target_node_path)
            };

            let variable_mapping = translation::Translation::match_path_variables(
                &schema_node_path,
                &target_node_path,
                &snippet,
                &target_schema,
            )
            .await?;

            log::debug!("variable_mapping: {:?}", variable_mapping);

            let target_node_path = target_node_path.with_mapped_variables(&variable_mapping);

            log::debug!("schema_node_path: {}", schema_node_path.to_string());
            log::debug!("target_node_path: {}", target_node_path.to_string());

            return Ok(Some((schema_node_path, target_node_path)));
        } else {
            log::warn!(
                "Schema node determined to be compatible but could not find target schema node"
            );
        }

        Ok(None)
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
        (
            u64 // tokens
        )
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

    pub async fn get_node_groups(
        meta_context: Arc<RwLock<MetaContext>>,
        acyclic_lineage: Lineage,
        lineage_subgroups: &HashMap<Lineage, Vec<Arc<Context>>>,
    ) -> Result<(
        NodeGroups,
        (u64,)
    ), Errors> {
        log::trace!("In get_node_groups");

        log::debug!("╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║                                                               ║");
        log::debug!("║                    GET NODE GROUPS START                      ║");
        log::debug!("║                                                               ║");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");

        let mut user_prompt = String::new();
        user_prompt.push_str(&format!("Acyclic Lineage: {}\n\n", acyclic_lineage.to_string()));

        let mut lineage_map: HashMap<String, Lineage> = HashMap::new();
        let mut indexed_lineage_map: HashMap<String, Lineage> = HashMap::new();

        for (lineage, subgroup) in lineage_subgroups {
            lineage_map.insert(lineage.to_string(), lineage.clone());

            user_prompt.push_str(&format!("{}\nLineage: {}\n{}\n\n", "#".repeat(100), lineage.to_string(), "#".repeat(100)));

            let sample: Vec<_> = subgroup.iter().take(12).collect();

            let common_lineages = if !sample.is_empty() {
                let mut common = read_lock!(sample[0].indexed_lineages).clone();
                for context in &sample[1..] {
                    let il = read_lock!(context.indexed_lineages);
                    common.retain(|l| il.contains(l));
                }
                common
            } else {
                Vec::new()
            };

            for (ctx_idx, context) in sample.iter().enumerate() {
                let document_node = read_lock!(context.document_node);
                user_prompt.push_str(&format!("{}\nContext[{}]: {}\n{}\n\n", "-".repeat(96), ctx_idx, document_node.to_string(), "-".repeat(96)));

                let indexed_lineages = read_lock!(context.indexed_lineages);
                let diverging: Vec<_> = indexed_lineages
                    .iter()
                    .filter(|l| !common_lineages.contains(l))
                    .collect();

                if !diverging.is_empty() {
                    user_prompt.push_str(&format!("Diverging indexed_lineages: {} entries\n", diverging.len()));
                    for (div_idx, div_lineage) in diverging.iter().enumerate() {
                        indexed_lineage_map.insert(div_lineage.to_string(), (*div_lineage).clone());
                        user_prompt.push_str(&format!("  [{}]: {}\n", div_idx, div_lineage.to_string()));
                    }
                    user_prompt.push_str("\n");
                }
            }
        }

        let (node_groups_data, metadata) = NodeAnalysis::get_node_groups(
            user_prompt,
        ).await?;

        let mut node_groups: NodeGroups = HashMap::new();

        for (lineage_str, classification) in node_groups_data.groups {
            let lineage = lineage_map.get(&lineage_str).cloned().ok_or_else(|| {
                Errors::LineageConversionError(format!("Lineage string not found in input: {}", lineage_str))
            })?;

            let node_group_classification = match classification {
                LineageClassification::Acyclic => NodeGroupClassification::Acyclic,
                LineageClassification::Uniform => NodeGroupClassification::Uniform,
                LineageClassification::Diverging(strings) => {
                    let lineages = strings.into_iter().map(|s| {
                        indexed_lineage_map.get(&s).cloned().ok_or_else(|| {
                            Errors::LineageConversionError(format!("Indexed lineage string not found in input: {}", s))
                        })
                    }).collect::<Result<Vec<Lineage>, Errors>>()?;

                    NodeGroupClassification::Diverging(lineages)
                }
            };

            node_groups.insert(lineage, node_group_classification);
        }

        Ok((node_groups, (metadata.tokens,)))
    }

    pub async fn function_to_operation(code: &str) -> Result<Option<()>, Errors> {
        openai::OpenAI::function_to_operation(&code).await
    }
}
