use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::basis_network::BasisNetwork;
use crate::graph_node::{Graph, GraphNode};
use crate::normal_context::NormalContext;
use crate::normal_meta_context::NormalMetaContext;
use crate::prelude::*;
use crate::traversal::{Traversal, TraversalKind};
use crate::xslt::Xslt;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraphMetadata {
    pub prompts: Vec<Hash>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraphNode {
    pub id: ID,
    pub parent: Option<Arc<RwLock<BasisGraphNode>>>,
    pub basis_network: Arc<BasisNetwork>,
    pub traversal: Option<Traversal>,
    pub children: Vec<Arc<RwLock<BasisGraphNode>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasisGraph {
    pub id: ID,
    pub name: Option<String>,
    pub description: Option<String>,
    pub lineage: Lineage,
    pub graph_roots: Vec<Arc<RwLock<BasisGraphNode>>>,
    pub metadata: BasisGraphMetadata,
}

impl BasisGraph {
    pub fn apply(
        &self,
        normalization_context: Arc<RwLock<NormalizationContext>>,
        parent: Graph,
    ) -> Result<NormalMetaContext, Errors> {
        log::trace!("In apply");

        let result = self
            .graph_roots
            .iter()
            .try_fold(
                None,
                |acc: Option<NormalMetaContext>,
                 graph_root|
                 -> Result<Option<NormalMetaContext>, Errors> {
                    let basis_network = &read_lock!(graph_root).basis_network;

                    let temp_parent = Arc::new(RwLock::new(GraphNode {
                        id: ID::new(),
                        parents: Vec::new(),
                        description: String::from("placeholder description"),
                        hash: Hash::new(),
                        subgraph_hash: Hash::new(),
                        lineage: Lineage::new(),
                        children: Vec::new(),
                    }));

                    let normal_meta_context = basis_network
                        .apply(Arc::clone(&normalization_context), Arc::clone(&temp_parent))?;

                    let children = read_lock!(graph_root).children.clone();

                    if children.is_empty() {
                        Self::augment(
                            Arc::clone(&normalization_context),
                            &normal_meta_context,
                            Arc::clone(&parent),
                        )?;
                    } else {
                        for child in children {
                            Self::hierarchize(
                                Arc::clone(&normalization_context),
                                &normal_meta_context,
                                Arc::clone(&child),
                                Arc::clone(&parent),
                            )?;
                        }
                    }

                    if let Some(acc) = acc {
                        let next_normal_meta_context = NormalMetaContext {
                            contexts: acc
                                .contexts
                                .into_iter()
                                .chain(normal_meta_context.contexts.clone())
                                .collect(),
                            graph_root: Arc::clone(&parent),
                            contexts_lookup: acc
                                .contexts_lookup
                                .into_iter()
                                .chain(normal_meta_context.contexts_lookup)
                                .collect(),
                        };

                        Ok(Some(next_normal_meta_context))
                    } else {
                        let next_normal_meta_context = NormalMetaContext {
                            contexts: normal_meta_context.contexts.clone(),
                            graph_root: Arc::clone(&parent),
                            contexts_lookup: normal_meta_context.contexts_lookup.clone(),
                        };

                        Ok(Some(next_normal_meta_context))
                    }
                },
            )?
            .unwrap();

        Ok(result)
    }

    fn augment(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        normal_meta_context: &NormalMetaContext,
        parent: Graph,
    ) -> Result<(), Errors> {
        let instances: Vec<Graph> = read_lock!(normal_meta_context.graph_root).children.clone();

        for instance in &instances {
            write_lock!(instance).parents = vec![Arc::clone(&parent)];
            write_lock!(parent).children.push(Arc::clone(&instance));
        }

        Ok(())
    }

    fn hierarchize(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        normal_meta_context: &NormalMetaContext,
        child: Arc<RwLock<BasisGraphNode>>,
        parent: Graph,
    ) -> Result<(), Errors> {
        let Some(traversal) = &read_lock!(child).traversal else {
            return Err(Errors::UnexpectedError(
                "Attempting to hierarchize a network without a Traversal".to_string(),
            ));
        };

        // We only need to find the parent instance to completely resolve instance hierarchy
        let xslt_rtl = {
            match &traversal.kind {
                TraversalKind::XPath { .. } => {
                    unimplemented!()
                }
                TraversalKind::Xslt { xslt_rtl, .. } => xslt_rtl.clone(),
            }
        };

        let xslt: Xslt = Xslt::new(&xslt_rtl)?;

        let instances: Vec<Graph> = read_lock!(normal_meta_context.graph_root).children.clone();

        for instance in &instances {
            write_lock!(instance).parents.clear();
        }

        for instance in &instances {
            let normal_context = normal_meta_context
                .contexts_lookup
                .get(&read_lock!(instance).id)
                .unwrap()
                .clone();

            let parent_instance = Self::get_parent(
                Arc::clone(&normalization_context),
                &xslt,
                normal_context.clone(),
                &normal_meta_context,
            )?;

            if let Some(parent_instance) = parent_instance {
                log::info!("Instance has a parent");

                write_lock!(parent_instance.graph_node)
                    .children
                    .push(Arc::clone(&instance));
                write_lock!(instance).parents = vec![Arc::clone(&parent_instance.graph_node)];
            } else {
                log::info!("Instance has no parent");
                write_lock!(instance).parents = vec![Arc::clone(&parent)];
                write_lock!(parent).children.push(Arc::clone(&instance));
            }
        }

        Ok(())
    }

    fn get_parent(
        normalization_context: Arc<RwLock<NormalizationContext>>,
        xslt: &Xslt,
        normal_context: Arc<NormalContext>,
        normal_meta_context: &NormalMetaContext,
    ) -> Result<Option<Arc<NormalContext>>, Errors> {
        let meta_context = {
            let lock = read_lock!(normalization_context);
            lock.meta_context
                .clone()
                .ok_or(Errors::DeficientNormalizationContextError(
                    "Meta context not provided in normalization context".to_string(),
                ))?
        };

        for context in &normal_context.contexts {
            let target_values = xslt.traverse(
                Arc::clone(&normalization_context),
                Arc::clone(&context.graph_node),
            )?;

            let Some(parent_graph_node) = target_values.into_iter().next().map(|v| v.graph) else {
                continue;
            };

            let target_context = meta_context
                .contexts_lookup
                .get(&read_lock!(parent_graph_node).id)
                .unwrap()
                .clone();

            if let Some(normal_context) = normal_meta_context
                .contexts_lookup
                .get(&target_context.id)
                .clone()
            {
                return Ok(Some(normal_context.clone()));
            }
        }

        Ok(None)
    }
}
