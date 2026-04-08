use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::prelude::*;
use crate::schema_node::SchemaNode;
use crate::xpath::{XPath, XPathAxis};

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

    pub fn traverse_using_xpath(&self, xpath: &XPath) -> Option<Graph> {

        let segments = &xpath.segments;

        if let Some((first, rest)) = segments.split_first() {

            match first.axis {
                XPathAxis::Child => unimplemented!(),
                XPathAxis::Parent => unimplemented!(),
                XPathAxis::Self_ => unimplemented!(),
                XPathAxis::Descendant => unimplemented!(),
                XPathAxis::Ancestor => unimplemented!(),
                XPathAxis::FollowingSibling => {

                    if self.parents.len() > 1 {
                        panic!("Why are we traversing a graph using xpath if nodes have more than one parent?");
                    }

                    if let Some(parent) = self.parents.first() {


                        if let Some(index_current) = read_lock!(parent).children.iter().position(|child| {
                            read_lock!(child).id == self.id
                        }) {

                            let target_index = index_current + 1;

                            if let Some(sibling) = read_lock!(parent).children.get(target_index) {


                                log::info!("Found sibling");


                                log::info!("node_test: {}", first.node_test);

                                log::info!("predicate: {:?}", first.predicate);



                            } else {
                                panic!("Sibling does not exist");
                            }

                        } else {
                            panic!("Could not find index of current node");
                        }


                    } else {
                        panic!("Trying to visit following sibling on a root node");
                    }


                },
                XPathAxis::PrecedingSibling => unimplemented!(),
            }

        }

        unimplemented!()
    }
}
