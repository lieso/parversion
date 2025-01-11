use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::task;
use futures::future;
use tokio::sync::Semaphore;

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
use crate::config::{CONFIG};

pub struct Analysis {
    context: Context,
    dataset: Dataset,
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn new<P: Provider>(provider: Arc<P>, input: AnalysisInput) -> Result<Self, Errors> {
        let mut context = Context::new();

        let dataset = input.to_dataset(provider.clone(), &mut context).await;
        let node_analysis = dataset.to_basis_nodes(provider.clone()).await;
        let network_analysis = node_analysis.to_basis_networks(provider.clone()).await;

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

pub struct AnalysisInput {
    document_node: DocumentNode,
    document_profile: Profile,
}

impl AnalysisInput {
    pub async fn from_document<P: Provider>(provider: Arc<P>, mut document: Document) -> Result<Self, Errors> {
        let profile = document.perform_analysis(provider).await?;

        Ok(AnalysisInput {
            document_node: document.get_document_node()?,
            document_profile: profile,
        })
    }

    pub async fn to_dataset<P: Provider>(self, provider: Arc<P>, context: &mut Context) -> Dataset {
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

struct Dataset {
    map: HashMap<ID, DataNode>,
    graph: Arc<RwLock<GraphNode>>,
}

impl Dataset {
    pub async fn to_basis_nodes<P: Provider>(&self, provider: Arc<P>) -> NodeAnalysis {
        log::trace!("In to_basis_nodes");

        let mut lineage_groups: HashMap<Lineage, Vec<DataNode>> = HashMap::new();

        for data_node in self.map.values() {
            let lineage = data_node.lineage.clone();

            lineage_groups
                .entry(lineage)
                .or_insert_with(Vec::new)
                .push(data_node.clone());
        }



        let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
        let semaphore = Arc::new(Semaphore::new(max_concurrency));

        let handles: Vec<_> = lineage_groups.into_iter()
            .map(|(lineage, group)| {

                let semaphore = semaphore.clone();
                let cloned_provider = provider.clone();

                task::spawn(async move {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    make_basis_node(cloned_provider, lineage, group).await
                })

            })
            .collect();





        let basis_nodes: Vec<BasisNode> = future::join_all(handles)
            .await
            .into_iter()
            .filter_map(|result| result.ok().and_then(Result::ok))
            .collect();



        unimplemented!()
    }
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

impl NodeAnalysis {
    pub async fn to_basis_networks<P: Provider>(&self, provider: Arc<P>) -> NetworkAnalysis {
        log::trace!("In to_basis_networks");

        unimplemented!()
    }
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
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

async fn make_basis_node<P: Provider>(
    provider: Arc<P>,
    lineage: Lineage,
    group: Vec<DataNode>
) -> Result<BasisNode, Errors> {
    log::trace!("In make_basis_node");


    unimplemented!()
}
