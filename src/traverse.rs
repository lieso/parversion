use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::meta_context::MetaContext;
use crate::context::{Context, ContextID};
use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::document::{Document, DocumentType};
use crate::document_format::{DocumentFormat};
use crate::profile::Profile;
use crate::provider::Provider;
use crate::json_node::JsonNode;

pub struct TraversalWithContext {
    pub nodeset: NodeSet,
    pub meta_context: MetaContext,
}

pub fn traverse_with_context(
    profile: &Profile,
    document: Document
) -> Result<TraversalWithContext, Errors> {
    log::trace!("In traverse_with_context");

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

    let meta_context = MetaContext {
        contexts,
        graph_root,
        document_root,
    };

    let traversal = TraversalWithContext {
        nodeset: NodeSet {
            data_nodes: data_nodes.values().cloned().collect()
        },
        meta_context,
    };

    Ok(traversal)
}

pub async fn build_document_from_nodeset<P: Provider>(
    provider: Arc<P>,
    nodeset: NodeSet,
    document_format: &Option<DocumentFormat>,
) -> Result<Document, Errors> {
    log::trace!("In build_document_from_nodeset");

    let data_nodes = nodeset.data_nodes;

    for data_node in data_nodes.into_iter() {
        let lineage = &data_node.lineage;

        if let Some(basis_node) = provider.get_basis_node_by_lineage(&lineage).await? {
            log::info!("Found basis node with lineage: {}", basis_node.lineage.to_string());

            let json_nodes: Vec<JsonNode> = basis_node.transformations
                .into_iter()
                .map(|transformation| {
                    transformation.transform(Arc::clone(&data_node))
                        .expect("Could not transform data node field")
                })
                .collect();

            log::debug!("json_nodes: {:?}", json_nodes);

        } else {
            log::warn!("basis node not found");
            //return Err(Errors::BasisNodeNotFound);
        }


    }

    unimplemented!()
}
