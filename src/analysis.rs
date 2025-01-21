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
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::profile::Profile;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::config::{CONFIG};

pub struct Analysis {
    dataset: Arc<Dataset>,
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn new<P: Provider>(
        provider: Arc<P>,
        input: AnalysisInput
    ) -> Result<Self, Errors> {
        let dataset = input.to_dataset(Arc::clone(&provider));
        let dataset = Arc::new(dataset);

        let node_analysis = Analysis::get_basis_nodes(
            Arc::clone(&provider),
            Arc::clone(&dataset)
        ).await?;
        let network_analysis = Analysis::get_basis_networks(
            Arc::clone(&provider),
            Arc::clone(&dataset)
        ).await?;

        let analysis = Analysis {
            dataset,
            node_analysis,
            network_analysis,
        };

        Ok(analysis)
    }

    pub fn to_document(self, document_format: &Option<DocumentFormat>) -> Result<Document, Errors> {
        unimplemented!()
    }

    async fn get_basis_nodes<P: Provider>(
        provider: Arc<P>,
        dataset: Arc<Dataset>
    ) -> Result<NodeAnalysis, Errors> {

        //let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
        let max_concurrency = 1;
        let semaphore = Arc::new(Semaphore::new(max_concurrency));

        let dataset_ref = Arc::clone(&dataset);
        let lineage_groups = dataset_ref.lineage_groups.clone();

        let handles: Vec<_> = lineage_groups.iter()
            .map(|(lineage, group)| {
                let semaphore = semaphore.clone();
                let cloned_lineage = lineage.clone();
                let cloned_group = group.clone();
                let cloned_provider = Arc::clone(&provider);
                let cloned_dataset = Arc::clone(&dataset_ref);

                task::spawn(async move {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    Analysis::get_basis_node(
                        cloned_provider,
                        cloned_dataset,
                        cloned_lineage,
                        cloned_group
                    ).await
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

    async fn get_basis_networks<P: Provider>(
        provider: Arc<P>,
        dataset: Arc<Dataset>
    ) -> Result<NetworkAnalysis, Errors> {
        unimplemented!()
    }

    async fn get_basis_node<P: Provider>(
        provider: Arc<P>,
        dataset: Arc<Dataset>,
        lineage: Lineage,
        group: Vec<ContextID>,
    ) -> Result<BasisNode, Errors> {
        log::trace!("In get_basis_node");

        if let Some(basis_node) = provider.get_basis_node_by_lineage(&lineage).await? {
            log::info!("Provider has supplied basis node");

            return Ok(basis_node);
        };


        unimplemented!()
    }
}

pub struct AnalysisInput {
    document_root: Arc<RwLock<DocumentNode>>,
    document_profile: Profile,
}

impl AnalysisInput {
    pub async fn from_document<P: Provider>(
        provider: Arc<P>,
        mut document: Document
    ) -> Result<Self, Errors> {
        let profile = document.perform_analysis(provider).await?;
        let document_node = document.get_document_node()?;

        Ok(AnalysisInput {
            document_root: Arc::new(RwLock::new(document_node.clone())),
            document_profile: profile,
        })
    }

    pub fn to_dataset<P: Provider>(
        self,
        provider: Arc<P>,
    ) -> Dataset {
        let mut document_nodes: HashMap<ContextID, Arc<RwLock<DocumentNode>>> = HashMap::new();
        let mut document_context: HashMap<DocumentNodeID, ContextID> = HashMap::new();

        let mut data_nodes: HashMap<ContextID, Arc<RwLock<DataNode>>> = HashMap::new();
        let mut data_context: HashMap<DataNodeID, ContextID> = HashMap::new();

        let mut graph_nodes: HashMap<ContextID, Arc<RwLock<GraphNode>>> = HashMap::new();
        let mut graph_context: HashMap<GraphNodeID, ContextID> = HashMap::new();

        let mut lineage_groups: HashMap<Lineage, Vec<ContextID>> = HashMap::new();

        traverse(
            &mut document_nodes,
            &mut document_context,
            &mut data_nodes,
            &mut data_context,
            &mut graph_nodes,
            &mut graph_context,
            &mut lineage_groups,
            Arc::clone(&self.document_root),
            &Lineage::new(),
            &self.document_profile,
            Vec::new(),
        );

        let root_node_document_id = read_lock!(self.document_root).id.clone();
        let root_node_context_id = data_context.get(&root_node_document_id).unwrap().clone();

        Dataset {
            data_nodes,
            data_context,
            graph_nodes,
            graph_context,
            document_nodes,
            document_context,
            lineage_groups,
            root_node_context_id,
        }
    }
}

type ContextID = ID;
type GraphNodeID = ID;
type DocumentNodeID = ID;
type DataNodeID = ID;

struct Dataset {
    data_nodes: HashMap<ContextID, Arc<RwLock<DataNode>>>,
    data_context: HashMap<DataNodeID, ContextID>,
    graph_nodes: HashMap<ContextID, Arc<RwLock<GraphNode>>>,
    graph_context: HashMap<GraphNodeID, ContextID>,
    document_nodes: HashMap<ContextID, Arc<RwLock<DocumentNode>>>, 
    document_context: HashMap<DocumentNodeID, ContextID>,
    lineage_groups: HashMap<Lineage, Vec<ContextID>>,
    root_node_context_id: ContextID,
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

fn traverse(
    document_nodes: &mut HashMap<ContextID, Arc<RwLock<DocumentNode>>>,
    document_context: &mut HashMap<DocumentNodeID, ContextID>,
    data_nodes: &mut HashMap<ContextID, Arc<RwLock<DataNode>>>,
    data_context: &mut HashMap<DataNodeID, ContextID>,
    graph_nodes: &mut HashMap<ContextID, Arc<RwLock<GraphNode>>>,
    graph_context: &mut HashMap<GraphNodeID, ContextID>,
    lineage_groups: &mut HashMap<Lineage, Vec<ContextID>>,
    document_node: Arc<RwLock<DocumentNode>>,
    parent_lineage: &Lineage,
    profile: &Profile,
    parents: Vec<Arc<RwLock<GraphNode>>>,
) -> Arc<RwLock<GraphNode>> {

    let context_id = ID::new();

    document_nodes.insert(context_id.clone(), Arc::clone(&document_node));
    document_context.insert(read_lock!(document_node).id.clone(), context_id.clone());

    let data_node = Arc::new(RwLock::new(DataNode::new(
        &profile.hash_transformation.clone().unwrap(),
        read_lock!(document_node).get_fields(),
        read_lock!(document_node).get_description(),
        parent_lineage,
    )));
    data_nodes.insert(context_id.clone(), Arc::clone(&data_node));
    data_context.insert(read_lock!(data_node).id.clone(), context_id.clone());

    let lineage = &read_lock!(data_node).lineage;
    lineage_groups
        .entry(lineage.clone())
        .or_insert_with(Vec::new)
        .push(context_id.clone());

    let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
        Arc::clone(&data_node),
        parents.clone()
    )));
    graph_nodes.insert(context_id.clone(), Arc::clone(&graph_node));
    graph_context.insert(read_lock!(graph_node).id.clone(), context_id.clone());

    {
        let children: Vec<Arc<RwLock<GraphNode>>> = read_lock!(document_node)
            .get_children(profile.xml_element_transformation.clone())
            .into_iter()
            .map(|child| {
                traverse(
                    document_nodes,
                    document_context,
                    data_nodes,
                    data_context,
                    graph_nodes,
                    graph_context,
                    lineage_groups,
                    Arc::new(RwLock::new(child)),
                    &lineage.clone(),
                    profile,
                    vec![Arc::clone(&graph_node)]
                )
            })
            .collect();

        let mut node_write_lock = graph_node.write().unwrap();
        node_write_lock.children.extend(children);
    }

    graph_node
}
