use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::prelude::*;
use crate::schema_node::SchemaNode;
use crate::xpath::{XPath, XPathAxis, XPathSegment};

pub type Graph = Arc<RwLock<GraphNode>>;
pub type GraphNodeID = ID;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GraphNode {
    pub id: ID,
    pub parents: Vec<Graph>,
    pub description: String,
    pub hash: Hash,
    pub subgraph_hash: Hash,
    pub lineage: Lineage,
    pub children: Vec<Graph>,
}

impl GraphNode {
    pub fn from_schema_node(schema_node: Arc<SchemaNode>, parents: Vec<Graph>) -> Self {
        GraphNode {
            id: ID::new(),
            parents,
            description: schema_node.description.clone(),
            hash: schema_node.hash.clone(),
            subgraph_hash: schema_node.hash.clone(),
            lineage: schema_node.lineage.clone(),
            children: Vec::new(),
        }
    }
    pub fn from_data_node(data_node: Arc<DataNode>, parents: Vec<Graph>) -> Self {
        let hash = data_node.hash.clone();

        GraphNode {
            id: ID::new(),
            parents,
            description: data_node.description.clone(),
            hash: hash.clone(),
            subgraph_hash: hash.clone(),
            lineage: data_node.lineage.clone(),
            children: Vec::new(),
        }
    }

    pub fn subgraph_hash(&self) -> Hash {
        let mut combined_hash = Hash::new();

        combined_hash.push(self.hash.to_string().unwrap_or_default());

        for child in &self.children {
            let child_read = read_lock!(child);
            let child_subgraph_hash = child_read.subgraph_hash();
            combined_hash.push(child_subgraph_hash.to_string().unwrap_or_default());
        }

        combined_hash.sort();
        combined_hash.finalize();

        combined_hash
    }

    pub fn traverse_using_xpath_segment(graph: Graph, xpath_segment: &XPathSegment) -> Result<Option<Graph>, Errors> {
        let lock = read_lock!(graph);

        if lock.parents.len() > 1 {
            return Err(Errors::XPathTraverseError("Why are we traversing a graph using xpath if nodes have more than one parent?".to_string()));
        }
        
        match xpath_segment.axis {
            XPathAxis::Child => unimplemented!(),
            XPathAxis::Parent => unimplemented!(),
            XPathAxis::Self_ => unimplemented!(),
            XPathAxis::Descendant => unimplemented!(),
            XPathAxis::Ancestor => unimplemented!(),
            XPathAxis::FollowingSibling => {
                if let Some(parent) = lock.parents.first() {
                    if let Some(index_current) = read_lock!(parent).children.iter().position(|child| {
                        read_lock!(child).id == lock.id
                    }) {

                        let target_index = index_current + 1;

                        if let Some(sibling) = read_lock!(parent).children.get(target_index) {


                            log::info!("Found sibling");


                            log::info!("node_test: {}", xpath_segment.node_test);

                            log::info!("predicate: {:?}", xpath_segment.predicate);

                            unimplemented!();

                        } else {
                            log::info!("Could not traverse to following sibling");
                            Ok(None)
                        }
                    } else {
                        Err(Errors::XPathTraverseError("Could not find index of current node as a child of parent".to_string()))
                    }
                } else {
                    Err(Errors::XPathTraverseError("Trying to visit following sibling on a root node".to_string()))
                }
            },
            XPathAxis::PrecedingSibling => unimplemented!(),
        }
    }

    pub fn traverse_using_xpath(start: Graph, xpath: &XPath) -> Result<Option<Graph>, Errors> {
        let segments = &xpath.segments;

        let mut current: Graph = Arc::clone(&start);
        let mut target: Option<Graph> = None;

        for segment in segments.iter() {
            if let Some(graph) = Self::traverse_using_xpath_segment(Arc::clone(&current), segment)? {
                current = Arc::clone(&graph);
            } else {
                return Ok(None);
            }
        }

        Ok(Some(current))
    }
}
