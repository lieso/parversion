use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::data_node::DataNode;
use crate::json_node::JsonNode;
use crate::basis_graph::{BasisGraph, BasisGraphBuilder};
use crate::document::{Document, DocumentType};
use crate::document_format::DocumentFormat;
use crate::transformation::{Transformation, HashTransformation};
use crate::provider::Provider;
use crate::context::Context;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::profile::Profile;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;

pub struct AnalysisInput {
    document_node: DocumentNode,
    document_profile: Profile,
}

struct Dataset {
    map: HashMap<ID, DataNode>,
    graph: Arc<RwLock<GraphNode>>,
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

pub struct Analysis {
    context: Context,
    dataset: Dataset,
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn new<P: Provider>(provider: &P, input: AnalysisInput) -> Result<Self, Errors> {
        let mut context = Context::new();

        let dataset = input.to_dataset(provider, &mut context).await;

        let node_analysis = dataset.to_basis_nodes(provider).await;

        let network_analysis = node_analysis.to_basis_networks(provider).await;

        Ok(Analysis {
            context,
            dataset,
            node_analysis,
            network_analysis,
        })
    }

    pub fn to_document(self, document_format: &Option<DocumentFormat>) -> Result<Document, Errors> {
        unimplemented!()
    }
}

impl AnalysisInput {
    pub async fn from_document<P: Provider>(provider: &P, mut document: Document) -> Result<Self, Errors> {
        let profile = document.perform_analysis(provider).await?;

        Ok(AnalysisInput {
            document_node: document.get_document_node()?,
            document_profile: profile,
        })
    }

    pub async fn to_dataset<P: Provider>(self, provider: &P, context: &mut Context) -> Dataset {
        let mut nodes: HashMap<ID, DataNode> = HashMap::new();

        let graph = traverse(
            &self.document_node,
            &Lineage::new(),
            context,
            &mut nodes,
            &self.document_profile
        );

        Dataset {
            map: nodes,
            graph,
        }
    }
}

impl Dataset {
    pub async fn to_basis_nodes<P: Provider>(&self, provider: &P) -> NodeAnalysis {
        unimplemented!()
    }
}

impl NodeAnalysis {
    pub async fn to_basis_networks<P: Provider>(&self, provider: &P) -> NetworkAnalysis {
        unimplemented!()
    }
}

fn traverse(
    document_node: &DocumentNode,
    parent_lineage: &Lineage,
    context: &mut Context,
    data_nodes: &mut HashMap<ID, DataNode>,
    profile: &Profile,
) -> Arc<RwLock<GraphNode>> {
    let context_id = context.register(document_node);

    let data_node = DataNode::new(
        &profile.hash_transformation.clone().unwrap(),
        context_id.clone(),
        document_node.get_fields(),
        document_node.get_description(),
        parent_lineage,
    );
    data_nodes.insert(data_node.id.clone(), data_node.clone());

    let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(&data_node)));

    {
        let children: Vec<Arc<RwLock<GraphNode>>> = document_node
            .get_children(profile.xml_element_transformation.clone().unwrap())
            .into_iter()
            .map(|child| {
                traverse(
                    &child,
                    &data_node.lineage,
                    context,
                    data_nodes,
                    profile,
                )
            })
            .collect();

        let mut node_write_lock = graph_node.write().unwrap();
        node_write_lock.children.extend(children);
    }

    graph_node
}
