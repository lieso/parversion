use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::data_node::{DataNode};
use crate::json_node::{JsonNode};
use crate::basis_graph::{BasisGraph, BasisGraphBuilder};
use crate::document::{Document, DocumentType};
use crate::document_format::{DocumentFormat};
use crate::transformation::{Transformation, HashTransformation};
use crate::provider::Provider;
use crate::context::Context;
use crate::document_node::DocumentNode;
use crate::graph_node::{GraphNode, Graph};
use crate::profile::Profile;
use crate::matrix::Matrix;

pub struct AnalysisInput {
    document_node: DocumentNode,
    document_profile: &Profile,
}

struct Dataset {
    map: HashMap<ID, DataNode>,
    graph: Arc<RwLock<GraphNode>>,
    //matrix: Matrix,
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

pub struct Analysis {
    context: Context,
}

impl Analysis {
    pub fn new(input: AnalysisInput) -> Self {
        let mut context = Context::new();

        let dataset = input.to_dataset(&mut context);

        let node_analysis = dataset.to_basis_nodes(&mut context);

        let network_analysis = node_analysis.to_basis_networks(&mut context);

        Analysis {
            context
        }
    }
}

impl AnalysisInput {
    pub async fn from_document(document: Document) -> Self {
        let profile = document.perform_analysis(provider).await?;

        AnalysisInput {
            document_node,
            document_profile,
        }
    }

    pub async fn to_dataset(self, context: &mut Context) -> Dataset {
        let mut nodes: HashMap<ID, DataNode> = HashMap::new();
        let mut matrix = Matrix::new();

        let graph = traverse(
            &self.document_node,
            &Lineage::new(),
            &mut context,
            &mut nodes,
            &self.profile
        );

        Dataset {
            nodes, graph
        }
    }
}

impl Dataset {
    pub async fn to_basis_nodes(self) -> NodeAnalysis {
        unimplemented!()
    }
}

impl NodeAnalysis {
    pub async fn to_basis_networks(self) -> NetworkAnalysis {
        unimplemented!()
    }
}









fn traverse(
    document_node: &DocumentNode,
    parent_lineage: &Lineage,
    context: &mut Context,
    data_nodes: &mut HashMap<ID, DataNode>,
    profile: &Profile,
) -> Graph {
    let context_id = context.register(document_node);

    let data_node = DataNode::new(
        profile.hash_transformation.unwrap(),
        context_id.clone(),
        document_node.get_fields(),
        document_node.get_description(),
        parent_lineage,
    );
    data_nodes.insert(data_node.id.clone(), data_node.clone());

    let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(&data_node)));

    { 
        let children: Vec<Arc<RwLock<GraphNode>>> = document_node
            .get_children(profile.document_transformations.unwrap())
            .iter()
            .map(|child| {
                traverse(
                    child.clone(),
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
