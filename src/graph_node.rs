use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use crate::data_node::DataNode;
use crate::prelude::*;
use crate::schema_node::SchemaNode;
use crate::xpath::{XPath, XPathAxis, XPathSegment, XPathPredicate};

pub type Graph = Arc<RwLock<GraphNode>>;
pub type GraphNodeID = ID;
pub type BottomUpIndexedLineages = Vec<Lineage>;

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

    pub fn index_in_parent(&self) -> Option<usize> {
        self.parents.first().and_then(|parent| {
            read_lock!(parent)
                .children
                .iter()
                .position(|child| read_lock!(child).id == self.id)
        })
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

    pub fn acyclic_subgraph_hash(&self) -> Hash {
        let mut combined_hash = Hash::new();

        combined_hash.push(self.hash.to_string().unwrap_or_default());

        for child in &self.children {
            let child_read = read_lock!(child);
            if child_read.lineage.is_cyclic() {
                continue;
            }
            let child_subgraph_hash = child_read.acyclic_subgraph_hash();
            combined_hash.push(child_subgraph_hash.to_string().unwrap_or_default());
        }

        combined_hash.sort();
        combined_hash.finalize();

        combined_hash
    }

    pub fn traverse_using_xpath_axis(
        meta_context: Arc<RwLock<MetaContext>>,
        graph: Graph,
        xpath_axis: &XPathAxis
    ) -> Result<Vec<Graph>, Errors> {
        log::debug!("xpath_axis: {:?}", xpath_axis);

        let lock = read_lock!(graph);

        if lock.parents.len() > 1 {
            return Err(Errors::XPathTraverseError("Why are we traversing a graph using xpath if nodes have more than one parent?".to_string()));
        }

        match xpath_axis {
            XPathAxis::Child => {
                Ok(lock.children.clone())
            },
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

                            Ok(vec![sibling.clone()])
                        } else {
                            log::info!("Could not traverse to following sibling");
                            Ok(vec![])
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

    pub fn traverse_using_xpath_node_test(
        meta_context: Arc<RwLock<MetaContext>>,
        graph: Graph,
        node_test: &String
    ) -> Result<Vec<Graph>, Errors> {
        log::debug!("node_test: {}", node_test);

        if node_test == "node()" {
            panic!("Received node_test 'node()'");
        }

        if node_test == "text()" {
            panic!("Received node_test 'text()'");
        }

        if node_test == "comment()" {
            panic!("Received node_test 'comment()'");
        }

        if node_test == "*" {
            panic!("Received node_test '*'");
        }

        let contexts = {
            let lock = read_lock!(meta_context);
            lock.contexts.clone().unwrap()
        };
        let current_context = contexts.get(&read_lock!(graph).id).unwrap();
        let document_node = current_context.document_node.clone();
        let name = read_lock!(document_node).get_element_name();

        if node_test.trim() == name.trim() {
            log::debug!("Graph passes node test");
            Ok(vec![graph.clone()])
        } else {
            Ok(vec![])
        }
    }

    pub fn traverse_using_xpath_predicate(
        meta_context: Arc<RwLock<MetaContext>>,
        graphs: Vec<Graph>,
        predicate: &XPathPredicate
    ) -> Result<Vec<Graph>, Errors> {
        log::debug!("predicate: {:?}, filtering {} graphs", predicate, graphs.len());

        match predicate {
            XPathPredicate::Position(index) => {
                log::debug!("Checking Position predicate, looking for index: {}", index);

                // XPath positions are 1-indexed

                if *index < 1 || *index as usize > graphs.len() {
                    log::debug!("Position {} out of range (have {} graphs)", index, graphs.len());
                    return Ok(vec![]);
                }

                let selected_graph = graphs.get(*index as usize - 1).cloned().unwrap();
                log::debug!("Position predicate matched graph at position {}", index);
                Ok(vec![selected_graph])
            }
            XPathPredicate::Attribute { name, value } => {
                log::debug!("Checking Attribute predicate: {}='{}'", name, value);

                let contexts = {
                    let lock = read_lock!(meta_context);
                    lock.contexts.clone().unwrap()
                };

                let filtered: Vec<Graph> = graphs
                    .iter()
                    .filter(|graph| {
                        let graph_id = read_lock!(graph).id.clone();
                        contexts
                            .get(&graph_id)
                            .and_then(|context| {
                                let foo = read_lock!(&context.document_node)
                                    .get_attribute_value(name);

                                read_lock!(&context.document_node)
                                    .get_attribute_value(name)
                                    .map(|attr_value| attr_value.trim() == value.trim())
                            })
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();

                log::debug!("Attribute predicate matched {} graphs", filtered.len());
                Ok(filtered)
            }
        }
    }

    pub fn traverse_using_xpath_segment(
        meta_context: Arc<RwLock<MetaContext>>,
        graph: Graph,
        xpath_segment: &XPathSegment
    ) -> Result<Vec<Graph>, Errors> {
        log::debug!("xpath_segment: {}", xpath_segment.to_string());

        let next_graphs: Vec<Graph> = Self::traverse_using_xpath_axis(
            Arc::clone(&meta_context),
            Arc::clone(&graph),
            &xpath_segment.axis
        )?;

        let next_graphs: Vec<Graph> = next_graphs
            .iter()
            .map(|graph| {
                Self::traverse_using_xpath_node_test(
                    Arc::clone(&meta_context),
                    Arc::clone(&graph),
                    &xpath_segment.node_test
                )
            })
            .collect::<Result<Vec<Vec<Graph>>, Errors>>()?
            .into_iter()
            .flatten()
            .collect();

        if let Some(predicate) = &xpath_segment.predicate {
            let next_graphs = Self::traverse_using_xpath_predicate(
                Arc::clone(&meta_context),
                next_graphs,
                &predicate
            )?;

            return Ok(next_graphs);
        }

        Ok(next_graphs)
    }

    pub fn traverse_using_xpath(
        meta_context: Arc<RwLock<MetaContext>>,
        start: Graph,
        xpath: &XPath
    ) -> Result<Option<Graph>, Errors> {
        log::debug!("xpath: {}", xpath.to_string());

        let segments = &xpath.segments;

        let mut current: Vec<Graph> = vec![Arc::clone(&start)];
        let mut target: Option<Graph> = None;

        for (index, segment) in segments.iter().enumerate() {
            current = current
                .iter()
                .map(|graph| {
                    Self::traverse_using_xpath_segment(Arc::clone(&meta_context), Arc::clone(graph), segment)
                })
                .collect::<Result<Vec<Vec<Graph>>, Errors>>()?
                .into_iter()
                .flatten()
                .collect();

            if current.is_empty() {
                if index == segments.len() - 1 {
                    log::info!("All segments processed, but a graph was not found");
                } else {
                    log::info!("No matches after segment {}, stopping early", index);
                }
                
                return Ok(None);
            }
        }

        if current.len() > 1 {
            panic!("We found more than one graph matching provided xpath expression");
        }

        Ok(current.first().cloned())
    }

    pub fn get_indexed_lineages(&self) -> BottomUpIndexedLineages {
        let mut ancestors = Vec::new();
        let mut current_id = self.id.clone();

        ancestors.push((self.id.clone(), self.hash.clone(), self.index_in_parent()));

        let mut remaining_parents = self.parents.clone();

        while !remaining_parents.is_empty() {
            let parent = read_lock!(remaining_parents[0]).clone();
            ancestors.push((parent.id.clone(), parent.hash.clone(), parent.index_in_parent()));
            remaining_parents = parent.parents.clone();
        }

        ancestors.reverse();

        let mut indexed_lineages = Vec::new();

        for inject_at_depth in 0..ancestors.len() {
            let mut lineage = Lineage::new();

            for (depth, (_, hash, index)) in ancestors.iter().enumerate() {
                if depth == inject_at_depth {
                    if let Some(idx) = index {
                        lineage = lineage.with_hash(Hash::from_str(&idx.to_string()));
                    }
                }
                lineage = lineage.with_hash(hash.clone());
            }

            indexed_lineages.push(lineage);
        }

        indexed_lineages
    }
}
