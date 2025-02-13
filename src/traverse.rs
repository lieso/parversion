use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::meta_context::MetaContext;
use crate::context::{Context, ContextID};
use crate::data_node::DataNode;
use crate::document_node::DocumentNode;
use crate::graph_node::{Graph, GraphNode};
use crate::document::{Document, DocumentType};
use crate::profile::Profile;

pub struct TraversalWithContext {
    data_nodes: HashMap<ID, Arc<DataNode>>,
    meta_context: MetaContext,
    contexts: HashMap<ContextID, Arc<Context>>,
}

pub fn traverse_with_context(
    profile: &Profile,
    document: Document
) -> Result<TraversalWithContext, Errors> {
    log::trace!("In traverse_with_context");

    let document_root = document.get_document_node()?;
    let document_root = Arc::new(RwLock::new(document_root.clone()));

    let mut data_nodes: HashMap<ID, Arc<DataNode>> = HashMap::new();
    let mut contexts: HashMap<ContextID, Arc<Context>> = HashMap::new();
    let mut context_ids: HashMap<ID, ContextID> = HashMap::new();

    fn recurse(
        document_node: Arc<RwLock<DocumentNode>>,
        data_nodes: &mut HashMap<ID, Arc<DataNode>>,
        parent_lineage: &Lineage,
        contexts: &mut HashMap<ContextID, Arc<Context>>,
        context_ids: &mut HashMap<ID, ContextID>,
        parents: Vec<Arc<RwLock<GraphNode>>>,
        profile: &Profile,
    ) -> Arc<RwLock<GraphNode>> {
        let data_node = Arc::new(
            DataNode::new(
                &profile.hash_transformation.clone().unwrap(),
                read_lock!(document_node).get_fields(),
                read_lock!(document_node).get_description(),
                parent_lineage,
            )
        );
        data_nodes.insert(data_node.id.clone(), Arc::clone(&data_node));

        let graph_node = Arc::new(RwLock::new(
            GraphNode::from_data_node(
                Arc::clone(&data_node),
                parents.clone(),
            )
        ));

        let context_id: ContextID = ID::new();
        contexts.insert(
            context_id.clone(),
            Arc::new(Context {
                id: context_id.clone(),
                document_node: Arc::clone(&document_node),
                graph_node: Arc::clone(&graph_node),
                data_node: Arc::clone(&data_node),
            })
        );

        context_ids.insert(data_node.id.clone(), context_id.clone());
        context_ids.insert(read_lock!(document_node).id.clone(), context_id.clone());
        context_ids.insert(read_lock!(graph_node).id.clone(), context_id.clone());

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
                        context_ids,
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
        &mut context_ids,
        Vec::new(),
        &profile
    );

    let meta_context = MetaContext {
        context_ids,
        graph_root,
        document_root,
    };

    let traversal = TraversalWithContext {
        data_nodes,
        meta_context,
        contexts,
    };

    Ok(traversal)
}
