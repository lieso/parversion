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
    dataset: Dataset,
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn new<P: Provider>(
        provider: Arc<P>,
        input: AnalysisInput
    ) -> Result<Self, Errors> {
        let dataset = input.to_dataset(Arc::clone(&provider));

        let node_analysis = self.get_basis_nodes(
            Arc::clone(&provider),
            &dataset
        ).await?;
        let network_analysis = self.get_basis_networks(
            Arc::clone(&provider),
            &dataset
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

    fn get_basis_nodes(provider: Arc<P>, dataset: &Dataset) {
        let mut lineage_groups: HashMap<Lineage, Vec<DataNode>> = HashMap::new();

        for data_node in self.map.values() {
            let lineage = data_node.lineage.clone();

            lineage_groups
                .entry(lineage)
                .or_insert_with(Vec::new)
                .push(data_node.clone());
        }



        //let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
        let max_concurrency = 1;
        let semaphore = Arc::new(Semaphore::new(max_concurrency));

        let handles: Vec<_> = lineage_groups.into_iter()
            .map(|(lineage, group)| {
                let semaphore = semaphore.clone();
                let cloned_provider = Arc::clone(&provider);
                let cloned_context = Arc::clone(&context);

                task::spawn(async move {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    get_basis_node(
                        cloned_provider,
                        cloned_context,
                        lineage,
                        group
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

    fn get_basis_networks(provider: Arc<P>, dataset: &Dataset) {
        unimplemented!()
    }

    async fn get_basis_node<P: Provider>(
        provider: Arc<P>,
        context: Arc<Context>,
        lineage: Lineage,
        group: Vec<DataNode>
    ) -> Result<BasisNode, Errors> {
        log::trace!("In get_basis_node");

        if let Some(basis_node) = provider.get_basis_node_by_lineage(&lineage).await? {
            log::info!("Provider has supplied basis node");

            return Ok(basis_node);
        }


        let data_node = group.first().unwrap();

        let c = context.get_snippet(&data_node.context_id);

        log::debug!("snippet: {}", c);


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

        let graph = traverse(
            &mut document_nodes,
            &mut document_context,
            &mut data_nodes,
            &mut data_context,
            &mut graph_nodes,
            &mut graph_context,
            &mut lineage_groups,
            Arc::clone(self.document_root),
            &Lineage::new(),
            &self.document_profile,
            Vec::new(),
        );

        let root_node_document_id = read_lock!(self.document_root).id.clone();
        let root_node_context_id = data_context.get(&root_node_document_id).unwrap();

        Dataset {
            data_nodes,
            data_context,
            graph_nodes,
            graph_context,
            document_nodes,
            document_context,
            root_node_context_id,
        }
    }
}

type ContextID = ID;
type GraphID = ID;
type DocumentNodeID = ID;
type DataNodeID = ID;

struct Dataset {
    data_nodes: HashMap<ContextID, Arc<RwLock<DataNode>>>,
    data_context: HashMap<DataNodeID, ContextID>,
    graph_nodes: HashMap<ContextID, Arc<RwLock<GraphNode>>,
    graph_context: HashMap<GraphID, ContextID>,
    document_nodes: HashMap<ContextID, Arc<RwLock<DocumentNode>>>, 
    document_context: HashMap<DocumentNodeID, ContextID>,
    lineage_groups: HashMap<Lineage, Vec<ContextID>>,
    root_node_context_id: ContextID,
    hash_maps: DatasetHashMaps,
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
    graph_nodes: &mut HashMap<ContextID, Arc<RwLock<GraphNode>>,
    graph_context: &mut HashMap<GraphID, ContextID>,
    lineage_groups: &mut HashMap<Lineage, Vec<ContextID>>,
    document_node: Arc<RwLock<DocumentNode>>>,
    parent_lineage: &Lineage,
    profile: &Profile,
    parents: Vec<Arc<RwLock<GraphNode>>>,
) {
    let context_id = ID::new();

    document_nodes.insert(context_id.clone(), Arc::clone(document_node));
    document_context.insert(read_lock!(document_node).id.clone(), context_id.clone());

    let data_node = Arc::new(RwLock::new(DataNode::new(
        &profile.hash_transformation.clone().unwrap(),
        document_node.get_fields(),
        document_node.get_description(),
        parent_lineage,
    )));
    data_nodes.insert(context_id.clone(), Arc::clone(data_node));
    data_context.insert(read_lock!(data_node).id.clone(), context_id.clone());

    let lineage = read_lock!(data_node).lineage.clone();
    lineage_groups
        .entry(lineage)
        .or_insert_with(Vec::new)
        .push(context_id.clone());

    let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
        Arc::clone(data_node),
        parents.clone()
    )));
    graph_nodes.insert(context_id.clone(), Arc::clone(graph_node));
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
                    Arc::clone(child),
                    &lineage,
                    profile,
                    vec![Arc::clone(&graph_node)]
                )
            })
            .collect();

        let mut node_write_lock = graph_node.write().unwrap();
        node_write_lock.children.extend(children);
    }
}
