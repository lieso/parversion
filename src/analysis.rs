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
use crate::context::Context;
use crate::llm::LLM;

pub struct Analysis {
    dataset: Arc<Dataset>,
    node_analysis: NodeAnalysis,
    network_analysis: NetworkAnalysis,
}

impl Analysis {
    pub async fn new<P: Provider>(
        provider: Arc<P>,
        input: &AnalysisInput
    ) -> Result<Self, Errors> {
        let dataset = input.clone().to_dataset(Arc::clone(&provider));
        let dataset = Arc::new(dataset);

        let meaningful_fields = input
            .document_profile
            .meaningful_fields
            .as_ref()
            .unwrap()
            .clone();
        let meaningful_fields = Arc::new(meaningful_fields);

        let node_analysis = Analysis::get_basis_nodes(
            Arc::clone(&provider),
            Arc::clone(&dataset),
            Arc::clone(&meaningful_fields)
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
        dataset: Arc<Dataset>,
        meaningful_fields: Arc<Vec<String>>
    ) -> Result<NodeAnalysis, Errors> {

        let max_concurrency = read_lock!(CONFIG).llm.max_concurrency;
        //let max_concurrency = 1;
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
                let cloned_meaningful_fields = Arc::clone(&meaningful_fields);

                task::spawn(async move {
                    let permit = semaphore.acquire_owned().await.unwrap();
                    Analysis::get_basis_node(
                        cloned_provider,
                        cloned_dataset,
                        cloned_lineage,
                        cloned_group,
                        cloned_meaningful_fields
                    ).await
                })

            })
            .collect();

        let basis_nodes: Vec<BasisNode> = future::join_all(handles)
            .await
            .into_iter()
            .filter_map(|result| result.ok().and_then(Result::ok))
            .filter_map(|opt| opt)
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
        group: Vec<KeyID>,
        meaningful_fields: Arc<Vec<String>>,
    ) -> Result<Option<BasisNode>, Errors> {
        log::trace!("In get_basis_node");
        log::debug!("lineage: {}", lineage.to_string());

        if let Some(basis_node) = provider.get_basis_node_by_lineage(&lineage).await? {
            log::info!("Provider has supplied basis node");

            return Ok(Some(basis_node));
        };

        let key_id = group.first().unwrap().clone();

        let data_node: Arc<RwLock<DataNode>> = dataset
            .data_nodes
            .get(&key_id)
            .unwrap()
            .clone();

        let meaningful_fields: Vec<String> = read_lock!(data_node)
            .fields
            .keys()
            .filter(|field| meaningful_fields.contains(&field))
            .cloned()
            .collect();

        if meaningful_fields.is_empty() {
            log::info!("Data node does not contain any meaningful information");

            return Ok(None);
        } else {
            let snippet = Context::generate_snippet(Arc::clone(&dataset), &key_id);


            let field = meaningful_fields.first().unwrap();

            let fields_transform = LLM::get_field_transformation(
                field.clone(),
                snippet,
            ).await;

            log::debug!("#####################################################################################################");
            log::debug!("#####################################################################################################");
            log::debug!("#####################################################################################################");

            log::debug!("fields_transform: {:?}", fields_transform);


        }

        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub struct AnalysisInput {
    pub document_root: Arc<RwLock<DocumentNode>>,
    pub document_profile: Profile,
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
        let mut document_nodes: HashMap<KeyID, Arc<RwLock<DocumentNode>>> = HashMap::new();
        let mut document_key: HashMap<DocumentNodeID, KeyID> = HashMap::new();

        let mut data_nodes: HashMap<KeyID, Arc<RwLock<DataNode>>> = HashMap::new();
        let mut data_key: HashMap<DataNodeID, KeyID> = HashMap::new();

        let mut graph_nodes: HashMap<KeyID, Arc<RwLock<GraphNode>>> = HashMap::new();
        let mut graph_key: HashMap<GraphNodeID, KeyID> = HashMap::new();

        let mut lineage_groups: HashMap<Lineage, Vec<KeyID>> = HashMap::new();

        traverse(
            &mut document_nodes,
            &mut document_key,
            &mut data_nodes,
            &mut data_key,
            &mut graph_nodes,
            &mut graph_key,
            &mut lineage_groups,
            Arc::clone(&self.document_root),
            &Lineage::new(),
            &self.document_profile,
            Vec::new(),
        );

        let root_node_document_id = read_lock!(self.document_root).id.clone();
        let root_node_key_id = document_key.get(&root_node_document_id).unwrap().clone();

        Dataset {
            data_nodes,
            data_key,
            graph_nodes,
            graph_key,
            document_nodes,
            document_key,
            lineage_groups,
            root_node_key_id,
        }
    }
}

pub type KeyID = ID;
pub type GraphNodeID = ID;
pub type DocumentNodeID = ID;
pub type DataNodeID = ID;

pub struct Dataset {
    pub data_nodes: HashMap<KeyID, Arc<RwLock<DataNode>>>,
    pub data_key: HashMap<DataNodeID, KeyID>,
    pub graph_nodes: HashMap<KeyID, Arc<RwLock<GraphNode>>>,
    pub graph_key: HashMap<GraphNodeID, KeyID>,
    pub document_nodes: HashMap<KeyID, Arc<RwLock<DocumentNode>>>, 
    pub document_key: HashMap<DocumentNodeID, KeyID>,
    pub lineage_groups: HashMap<Lineage, Vec<KeyID>>,
    pub root_node_key_id: KeyID,
}

struct NodeAnalysis {
    basis_nodes: Vec<BasisNode>,
}

struct NetworkAnalysis {
    basis_networks: Vec<BasisNetwork>,
}

fn traverse(
    document_nodes: &mut HashMap<KeyID, Arc<RwLock<DocumentNode>>>,
    document_key: &mut HashMap<DocumentNodeID, KeyID>,
    data_nodes: &mut HashMap<KeyID, Arc<RwLock<DataNode>>>,
    data_key: &mut HashMap<DataNodeID, KeyID>,
    graph_nodes: &mut HashMap<KeyID, Arc<RwLock<GraphNode>>>,
    graph_key: &mut HashMap<GraphNodeID, KeyID>,
    lineage_groups: &mut HashMap<Lineage, Vec<KeyID>>,
    document_node: Arc<RwLock<DocumentNode>>,
    parent_lineage: &Lineage,
    profile: &Profile,
    parents: Vec<Arc<RwLock<GraphNode>>>,
) -> Arc<RwLock<GraphNode>> {

    let key_id = ID::new();

    document_nodes.insert(key_id.clone(), Arc::clone(&document_node));
    document_key.insert(read_lock!(document_node).id.clone(), key_id.clone());

    let data_node = Arc::new(RwLock::new(DataNode::new(
        &profile.hash_transformation.clone().unwrap(),
        read_lock!(document_node).get_fields(),
        read_lock!(document_node).get_description(),
        parent_lineage,
    )));
    data_nodes.insert(key_id.clone(), Arc::clone(&data_node));
    data_key.insert(read_lock!(data_node).id.clone(), key_id.clone());

    let lineage = &read_lock!(data_node).lineage;
    lineage_groups
        .entry(lineage.clone())
        .or_insert_with(Vec::new)
        .push(key_id.clone());

    let graph_node = Arc::new(RwLock::new(GraphNode::from_data_node(
        Arc::clone(&data_node),
        parents.clone()
    )));
    graph_nodes.insert(key_id.clone(), Arc::clone(&graph_node));
    graph_key.insert(read_lock!(graph_node).id.clone(), key_id.clone());

    {
        let children: Vec<Arc<RwLock<GraphNode>>> = read_lock!(document_node)
            .get_children(profile.xml_element_transformation.clone())
            .into_iter()
            .map(|child| {
                traverse(
                    document_nodes,
                    document_key,
                    data_nodes,
                    data_key,
                    graph_nodes,
                    graph_key,
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
