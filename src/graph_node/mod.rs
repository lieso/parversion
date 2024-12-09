use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use tokio::sync::Semaphore;
use tokio::task;
use serde_json::{
    json,
    to_string_pretty,
    Result as SerdeResult,
    Value as JsonValue,
    from_str
};

mod debug;
mod analysis;

use crate::xml_node::{XmlNode};
use crate::xml_node;
use crate::basis_node::{BasisNode};
use crate::basis_graph::{BasisGraph};
use crate::constants;
use crate::macros::*;
use crate::graph_node::analysis::*;
use crate::basis_graph::{Subgraph};
use crate::config::{CONFIG};
use crate::utility;

#[derive(Clone, Debug)]
pub struct GraphNode<T: GraphNodeData> {
    pub id: String,
    pub hash: String,
    pub lineage: String,
    pub parents: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub children: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub data: T,
}

pub type Graph<T> = Arc<RwLock<GraphNode<T>>>;

pub trait GraphNodeData: Clone + Send + Sync + std::fmt::Debug + Serialize + for<'de> Deserialize<'de> {
    fn new(description: String) -> Self;
    fn describe(&self) -> String;
}

impl<T: GraphNodeData> GraphNode<T> {
    pub fn serialize(&self) -> SerdeResult<String> {
        let mut visited = HashSet::new();
        let json_value = self.serialize_node(&mut visited)?;
        to_string_pretty(&json_value)
    }

    pub fn deserialize(json_str: &str) -> SerdeResult<Graph<T>> {
        let json_value: JsonValue = from_str(json_str)?;
        let mut visited = HashMap::new();
        Self::deserialize_node(&json_value, &mut visited)
    }

    fn deserialize_node(
        json_value: &JsonValue,
        visited: &mut HashMap<String, Graph<T>>,
    ) -> SerdeResult<Graph<T>> {
        let id = json_value["id"].as_str().unwrap().to_string();
        if let Some(existing_node) = visited.get(&id) {
            return Ok(Arc::clone(existing_node));
        }

        let data: T = serde_json::from_value(json_value["data"].clone())?;

        let temp_node = Arc::new(RwLock::new(GraphNode {
            id: id.clone(),
            hash: json_value["hash"].as_str().unwrap().to_string(),
            lineage: json_value["lineage"].as_str().unwrap().to_string(),
            data,
            parents: Vec::new(),
            children: Vec::new(),
        }));
        visited.insert(id.clone(), Arc::clone(&temp_node));

        let default_parents = vec![];
        let parents_json = json_value["parents"].as_array().unwrap_or(&default_parents);
        let parents: SerdeResult<Vec<_>> = parents_json
            .iter()
            .map(|parent_json| Self::deserialize_node(parent_json, visited))
            .collect();

        let default_children = vec![];
        let children_json = json_value["children"].as_array().unwrap_or(&default_children);
        let children: SerdeResult<Vec<_>> = children_json
            .iter()
            .map(|child_json| Self::deserialize_node(child_json, visited))
            .collect();

        {
            let mut node = temp_node.write().unwrap();
            node.parents = parents?;
            node.children = children?;
        }

        Ok(temp_node)
    }

    fn serialize_node(&self, visited: &mut HashSet<String>) -> SerdeResult<serde_json::Value> {
        if visited.contains(&self.id) {
            return Ok(json!({"id": self.id, "hash": self.hash }));
        }

        visited.insert(self.id.clone());

        let parents_json: SerdeResult<Vec<_>> = self
            .parents
            .iter()
            .map(|parent| read_lock!(parent).serialize_node(visited))
            .collect();

        let children_json: SerdeResult<Vec<_>> = self
            .children
            .iter()
            .map(|child| read_lock!(child).serialize_node(visited))
            .collect();

        Ok(json!({
            "id": self.id,
            "hash": self.hash,
            "lineage": self.lineage,
            "data": self.data,
            "parents": parents_json?,
            "children": children_json?,
        }))
    }
}

pub fn get_depth(node: Graph<XmlNode>) -> usize {
    log::trace!("In get_depth");

    let mut depth = 0;

    let mut current = node;

    loop {
        let parent_nodes = current.read().unwrap().parents.clone();
        if parent_nodes.is_empty() {
            break;
        }

        current = Arc::clone(&parent_nodes[0]);
        depth += 1;
    }

    depth
}

pub fn build_tree(document: Document) -> Graph<XmlNode> {
    let mut reader = std::io::Cursor::new(document.value);
    let xml = XmlNode::parse(&mut reader).expect("Could not parse XML");

    GraphNode::from_xml(&xml, Vec::new(), Vec::new())
}

pub fn build_unique_graph(document: Document) -> Graph<XmlNode> {
    let graph = build_tree(xml);

    cyclize(Arc::clone(&graph));
    log::info!("Done cyclizing input graph");

    prune(Arc::clone(&graph));
    log::info!("Done pruning input graph");
}

pub fn to_xml_string(graph: Graph<XmlNode>) -> String {
    let mut visited: HashSet<String> = HashSet::new();
    let mut xml = String::new();

    fn recurse(
        node: Graph<XmlNode>,
        xml: &mut String,
        visited: &mut HashSet<String>,
    ) {
        let xml_node: &XmlNode = &read_lock!(node).data;

        if visited.contains(&read_lock!(node).id) {
            return;
        }

        if xml_node.is_element() {
            let opening_tag = xml_node.get_opening_tag();
            let closing_tag = xml_node.get_closing_tag();

            xml.push_str(&opening_tag);

            visited.insert(read_lock!(node).id.clone());

            for child in read_lock!(node).children.iter() {
                recurse(
                    Arc::clone(&child),
                    xml,
                    visited,
                );
            }

            visited.remove(&read_lock!(node).id);

            xml.push_str(&closing_tag);
        }

        if let Some(text) = &xml_node.text {
            xml.push_str(&text.clone());
        }
    }

    recurse(
        Arc::clone(&graph),
        &mut xml,
        &mut visited
    );

    xml
}

impl GraphNode<XmlNode> {
    fn from_xml(
        xml: &XmlNode,
        parents: Vec<Graph<XmlNode>>,
        lineage: Vec<String>
    ) -> Graph<XmlNode> {
        let hash = xml_node::xml_to_hash(xml);

        let mut new_lineage = lineage.clone();
        new_lineage.push(hash.clone());

        let mut hasher = Sha256::new();
        let mut hasher_items = utility::remove_duplicate_sequences(new_lineage.clone());
        hasher.update(hasher_items.join(""));

        let lineage_hash = format!("{:x}", hasher.finalize());

        let node = Arc::new(RwLock::new(GraphNode {
            id: ID::new(),
            hash,
            lineage: lineage_hash,
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        {
            let children: Vec<Graph<XmlNode>> = xml
                .get_children()
                .iter()
                .map(|child| {
                    GraphNode::from_xml(child, vec![node.clone()], new_lineage.clone())
                })
                .collect();

            let mut node_write_lock = node.write().unwrap();
            node_write_lock.children.extend(children);
        }

        node
    }
}

pub fn graph_hash<T: GraphNodeData>(graph: Graph<T>) -> String {
    let mut visited: HashSet<String> = HashSet::new();

    fn compute_hash<T: GraphNodeData>(
        node: Graph<T>,
        visited: &mut HashSet<String>,
    ) -> String {
        let node = node.read().unwrap();

        if visited.contains(&node.id) {
            return "cycle".to_owned();
        }

        let mut hash = Hash::with_items(vec![node.hash.clone()]);

        visited.insert(node.id.clone());

        for child in node.children.iter() {
            hash.push(compute_hash(child.clone(), visited));
        }

        visited.remove(&node.id);

        hash.sort().finalize().to_string()
    }

    compute_hash(graph, &mut visited)
}

pub fn bft<T: GraphNodeData>(graph: Graph<T>, visit: &mut dyn FnMut(Graph<T>) -> bool) {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(graph);

    while let Some(current) = queue.pop_front() {
        if !visited.insert(read_lock!(current).id.clone()) {
            continue;
        }

        if !visit(Arc::clone(&current)) {
            break;
        }

        for child in read_lock!(current).children.iter() {
            queue.push_back(child.clone());
        }
    }
}

pub fn cyclize<T: GraphNodeData>(graph: Graph<T>) {
    log::trace!("In cyclize");

    fn dfs<T: GraphNodeData>(
        node: Graph<T>,
        visited: &mut HashMap<String, Graph<T>>
    ) {
        let node_id = read_lock!(node).id.clone();
        let node_hash = read_lock!(node).hash.clone();

        if let Some(first_occurrence) = visited.get(&node_hash) {
            log::info!("Detected cycle");

            let parents = {
                let read_lock = read_lock!(node);
                read_lock.parents.clone()
            };

            for parent in parents.iter() {
                let children_to_retain: Vec<_> = {
                    let read_lock = read_lock!(parent);
                    read_lock.children
                        .iter()
                        .filter(|child| read_lock!(child).id != node_id)
                        .cloned()
                        .collect()
                };

                let mut write_lock = write_lock!(parent);
                write_lock.children = children_to_retain;
                if !write_lock.children.iter().any(|c| Arc::ptr_eq(c, first_occurrence)) {
                    write_lock.children.push(first_occurrence.clone());
                }
            }

            let children = {
                let read_lock = read_lock!(node);
                read_lock.children.clone()
            };
            for child in children.iter() {
                let child_clone = child.clone();

                let parents_to_retain: Vec<_> = {
                    let read_lock = read_lock!(child);
                    read_lock.parents
                        .iter()
                        .filter(|parent| read_lock!(parent).id != node_id)
                        .cloned()
                        .collect()
                };

                {
                    let mut write_lock = write_lock!(child);
                    write_lock.parents = parents_to_retain;
                    if !write_lock.parents.iter().any(|p| Arc::ptr_eq(p, first_occurrence)) {
                        write_lock.parents.push(first_occurrence.clone());
                    }
                }

                {
                    let mut write_lock = write_lock!(first_occurrence);
                    if !write_lock.children.iter().any(|c| Arc::ptr_eq(c, &child_clone)) {
                        write_lock.children.push(child_clone);
                    }
                }
            }

            {
                let mut write_lock = write_lock!(node);
                write_lock.parents = Vec::new();
                write_lock.children = Vec::new();
            }
        } else {
            visited.insert(node_hash.clone(), Arc::clone(&node));

            let children: Vec<Graph<T>>;
            {
                let read_lock = read_lock!(node);
                children = read_lock.children.clone();
            }

            for child in children.iter() {
                dfs(
                    child.clone(),
                    visited,
                );
            }

            visited.remove(&node_hash);
        }
    }

    dfs(
        graph,
        &mut HashMap::new(),
    );
}

pub fn prune(graph: Graph<XmlNode>) {
    log::trace!("In prune");

    fn is_twin(a: Graph<XmlNode>, b: Graph<XmlNode>) -> bool {
        let a_rl = a.read().unwrap();
        let b_rl = b.read().unwrap();

        a_rl.id != b_rl.id && a_rl.lineage == b_rl.lineage
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(graph);

    while let Some(parent) = queue.pop_front() {
        if !visited.insert(read_lock!(parent).id.clone()) {
            continue;
        }

        {
            let children_to_retain: Vec<_> = {
                let parent_read = read_lock!(parent);
                parent_read.children
                    .iter()
                    .filter(|child| {
                        if read_lock!(child).data.is_element() {
                            return !read_lock!(child).children.is_empty();
                        }

                        true
                    })
                    .cloned()
                    .collect()
            };

            write_lock!(parent).children = children_to_retain;
        }

        loop {
            let children: Vec<Graph<XmlNode>> = read_lock!(parent).children.clone();

            let maybe_twins: Option<(Graph<XmlNode>, Graph<XmlNode>)> = children
                .iter()
                .find_map(|child| {
                    children
                        .iter()
                        .find(|sibling| is_twin(Arc::clone(child), Arc::clone(sibling)))
                        .map(|sibling| (Arc::clone(child), Arc::clone(sibling)))
                });

            if let Some(twins) = maybe_twins {
                log::info!("Found two siblings nodes that are twins");
                merge_nodes(Arc::clone(&parent), twins);
                //visited = HashSet::new(); // why?
            } else {
                break;
            }
        }

        for child in read_lock!(parent).children.iter() {
            queue.push_back(child.clone());
        }
    }
}

pub fn build_xml_with_target_node(
    output_tree: Graph<XmlNode>,
    target_node: Graph<XmlNode>
) -> (
    String, // html before target node
    String, // target node opening tag
    String, // target node child content
    String, // target node closing tag
    String, // html after target node
) {
    log::trace!("In build_xml_with_target_node");

    let mut before_html = String::new();
    let mut target_opening_html = String::new();

    let mut target_child_content = String::new();
    let mut target_closing_html = String::new();
    let mut after_html = String::new();
    let mut found_target = false;

    fn recurse(
        current: Graph<XmlNode>,
        target: Graph<XmlNode>,
        found_target: &mut bool,
        before_html: &mut String,
        target_opening_html: &mut String,
        target_child_content: &mut String,
        target_closing_html: &mut String,
        after_html: &mut String,
    ) {
        let xml_node: &XmlNode = &read_lock!(current).data;

        if xml_node.is_element() {
            let opening_tag = xml_node.get_opening_tag();
            let closing_tag = xml_node.get_closing_tag();

            if *found_target {
                after_html.push_str(&opening_tag);
            } else if read_lock!(current).id == read_lock!(target).id {
                *found_target = true;
                target_opening_html.push_str(&opening_tag);
            } else {
                before_html.push_str(&opening_tag);
            }

            for child in read_lock!(current).children.iter() {
                recurse(
                    Arc::clone(&child),
                    Arc::clone(&target),
                    found_target,
                    before_html,
                    target_opening_html,
                    target_child_content,
                    target_closing_html,
                    after_html,
                );
            }

            if *found_target && read_lock!(current).id == read_lock!(target).id {
                target_closing_html.push_str(&closing_tag);
            } else if *found_target {
                after_html.push_str(&closing_tag);
            } else {
                before_html.push_str(&closing_tag);
            }
        }

        if let Some(text) = &xml_node.text {
            if *found_target {
                after_html.push_str(&text.clone());
            } else if read_lock!(current).id == read_lock!(target).id {
                *found_target = true;
                target_child_content.push_str(&text.clone());
            } else {
                before_html.push_str(&text.clone());
            }
        }
    }

    recurse(
        Arc::clone(&output_tree),
        Arc::clone(&target_node),
        &mut found_target,
        &mut before_html,
        &mut target_opening_html,
        &mut target_child_content,
        &mut target_closing_html,
        &mut after_html
    );

    (
        before_html,
        target_opening_html,
        target_child_content,
        target_closing_html,
        after_html
    )
}

fn merge_nodes<T: GraphNodeData>(parent: Graph<T>, nodes: (Graph<T>, Graph<T>)) {
    log::trace!("In merge_nodes");

    let keep_node = nodes.0;
    let discard_node = nodes.1;

    write_lock!(discard_node).parents.clear();

    let discard_node_id = read_lock!(discard_node).id.clone();
    let discard_children: Vec<_> = read_lock!(discard_node).children.clone();

    for child in discard_children {
        let mut child_write = write_lock!(child);

        child_write.parents.retain(|p| !Arc::ptr_eq(p, &discard_node));

        if !child_write.parents.iter().any(|p| Arc::ptr_eq(p, &keep_node)) {
            child_write.parents.push(Arc::clone(&keep_node));
        }
        
        let mut keep_node_write = write_lock!(keep_node);
        if !keep_node_write.children.iter().any(|c| Arc::ptr_eq(c, &child)) {
            keep_node_write.children.push(Arc::clone(&child));
        }
    }

    let children_to_retain: Vec<_> = {
        let parent_read = read_lock!(parent);
        parent_read.children
            .iter()
            .filter(|child| read_lock!(child).id != discard_node_id)
            .cloned()
            .collect()
    };

    write_lock!(parent).children = children_to_retain;
}
