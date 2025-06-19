use std::collections::{HashSet, HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use serde_json::{json, Value};

use crate::prelude::*;
use crate::meta_context::MetaContext;
use crate::context::{Context};
use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::document::{Document, DocumentType, DocumentMetadata};
use crate::document_format::{DocumentFormat};
use crate::profile::Profile;
use crate::provider::Provider;

pub fn traverse_document(
    document: Document,
    meta_context: Arc<RwLock<MetaContext>>,
) -> Result<(
    HashMap<ID, Arc<Context>>, // context
    Arc<RwLock<GraphNode>> // graph root
), Errors> {
    log::trace!("In traverse_document");

    let lock = read_lock!(meta_context);
    let profile = lock.profile.as_ref().ok_or(Errors::ProfileNotProvided)?;

    let document_root = document.get_document_node()?;
    let document_root = Arc::new(RwLock::new(document_root.clone()));

    let mut data_nodes: HashMap<ID, Arc<DataNode>> = HashMap::new();
    let mut contexts: HashMap<ID, Arc<Context>> = HashMap::new();

    fn recurse(
        document_node: Arc<RwLock<DocumentNode>>,
        data_nodes: &mut HashMap<ID, Arc<DataNode>>,
        parent_lineage: &Lineage,
        contexts: &mut HashMap<ID, Arc<Context>>,
        parents: Vec<Arc<RwLock<GraphNode>>>,
        profile: &Profile,
    ) -> Arc<RwLock<GraphNode>> {
        let data_node = Arc::new(
            DataNode::new(
                profile.meaningful_fields.clone().unwrap(),
                &profile.hash_transformation.clone().unwrap(),
                read_lock!(document_node).get_fields(),
                read_lock!(document_node).get_description(),
                parent_lineage,
            )
        );

        let graph_node = Arc::new(RwLock::new(
            GraphNode::from_data_node(
                Arc::clone(&data_node),
                parents.clone(),
            )
        ));

        let context = Arc::new(Context {
            id: ID::new(),
            lineage: data_node.lineage.clone(),
            document_node: Arc::clone(&document_node),
            graph_node: Arc::clone(&graph_node),
            data_node: Arc::clone(&data_node),
        });

        data_nodes.insert(data_node.id.clone(), Arc::clone(&data_node));

        contexts.insert(data_node.id.clone(), Arc::clone(&context));
        contexts.insert(read_lock!(document_node).id.clone(), Arc::clone(&context));
        contexts.insert(read_lock!(graph_node).id.clone(), Arc::clone(&context));

        {
            let children: Vec<Arc<RwLock<GraphNode>>> = read_lock!(document_node)
                .get_children(profile.xml_element_transformation.clone())
                .into_iter()
                .map(|child| {
                    recurse(
                        Arc::new(RwLock::new(child)),
                        data_nodes,
                        &data_node.lineage,
                        contexts,
                        vec![Arc::clone(&graph_node)],
                        profile
                    )
                })
                .collect();

            let mut write_lock = graph_node.write().unwrap();

            let child_hashes: Vec<Hash> = children.iter()
                .map(|child| read_lock!(child).hash.clone())
                .collect();

            let mut subgraph_hash = Hash::from_items(child_hashes.clone());
            let subgraph_hash = subgraph_hash
                .sort()
                .push(write_lock.hash.clone())
                .finalize();

            write_lock.subgraph_hash = subgraph_hash.clone();
            write_lock.children.extend(children);
        }

        graph_node
    }

    let graph_root = recurse(
        Arc::clone(&document_root),
        &mut data_nodes,
        &Lineage::new(),
        &mut contexts,
        Vec::new(),
        &profile
    );

    Ok((contexts, graph_root))
}


