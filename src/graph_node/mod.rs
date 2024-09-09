use serde::{Serialize, Serializer, Deserialize};
use serde_json::{json, to_string_pretty, Result as SerdeResult};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use tokio::sync::Semaphore;
use tokio::task;

mod debug;
mod analysis;

use crate::xml_node::{XmlNode};
use crate::xml_node;
use crate::basis_node::{BasisNode};
use crate::constants;
use crate::macros::*;
use crate::graph_node::analysis::*;

#[derive(Deserialize, Clone, Debug)]
pub struct GraphNode<T: GraphNodeData> {
    pub id: String,
    pub hash: String,
    pub parents: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub children: Vec<Arc<RwLock<GraphNode<T>>>>,
    pub data: T,
}

pub type Graph<T> = Arc<RwLock<GraphNode<T>>>;

pub trait GraphNodeData: Clone + Send + Sync {
    fn new(description: String) -> Self;
    fn describe(&self) -> String;
}

pub fn build_graph(xml: String) -> Arc<RwLock<GraphNode<XmlNode>>> {
    let mut reader = std::io::Cursor::new(xml);
    let xml = XmlNode::parse(&mut reader).expect("Could not parse XML");

    GraphNode::from_xml(&xml, Vec::new())
}

impl<T: GraphNodeData> GraphNode<T> {
    pub fn serialize(&self) -> SerdeResult<String> {
        let mut visited = HashSet::new();
        let json_value = self.serialize_node(&mut visited)?;
        to_string_pretty(&json_value)
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
            "data": self.data.describe(),
            "parents": parents_json?,
            "children": children_json?,
        }))
    }
}

impl GraphNode<XmlNode> {
    fn from_xml(xml: &XmlNode, parents: Vec<Graph<XmlNode>>) -> Graph<XmlNode> {
        let node = Arc::new(RwLock::new(GraphNode {
            id: Uuid::new_v4().to_string(),
            hash: xml_node::xml_to_hash(xml),
            parents,
            children: Vec::new(),
            data: xml.without_children(),
        }));

        {
            let children: Vec<Graph<XmlNode>> = xml
                .get_children()
                .iter()
                .map(|child| {
                    GraphNode::from_xml(child, vec![node.clone()])
                })
                .collect();

            let mut node_write_lock = node.write().unwrap();
            node_write_lock.children.extend(children);
        }

        node
    }
}

impl<T: GraphNodeData> GraphNode<T> {
    pub fn from_void() -> Graph<T> {
        Arc::new(RwLock::new(GraphNode {
            id: Uuid::new_v4().to_string(),
            hash: constants::ROOT_NODE_HASH.to_string(),
            parents: Vec::new(),
            children: Vec::new(),
            data: T::new("blank".to_string()),
        }))
    }

    pub fn is_linear(&self) -> bool {
        self.children.len() == 1 && self.parents.len() == 1
    }

    pub fn is_linear_head(&self) -> bool {
        if self.is_linear() {
            let parent = self.parents.first().unwrap();
            return !read_lock!(parent).is_linear();
        }

        false
    }

    pub fn is_linear_tail(&self) -> bool {
        self.is_linear() && !self.is_linear_head()
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

        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(node.hash.clone());

        visited.insert(node.id.clone());

        for child in node.children.iter() {
            hasher_items.push(compute_hash(child.clone(), visited));
        }

        visited.remove(&node.id);

        hasher_items.sort();
        hasher.update(hasher_items.join(""));

        format!("{:x}", hasher.finalize())
    }

    compute_hash(graph, &mut visited)
}

pub fn deep_copy<T: GraphNodeData, U: GraphNodeData>(graph: Graph<U>, parents: Vec<Graph<T>>) -> Graph<T> where T: GraphNodeData {
    log::trace!("In deep_copy");

    let guard = graph.read().unwrap();

    let new_node = Arc::new(RwLock::new(GraphNode {
        id: guard.id.clone(),
        hash: guard.hash.clone(),
        parents,
        children: Vec::new(),
        data: T::new(guard.data.describe()),
    }));

    {
        let children: Vec<Graph<T>> = guard.children.iter()
            .map(|child| deep_copy(child.clone(), vec![new_node.clone()]))
            .collect();
        let mut node_write_lock = new_node.write().unwrap();
        node_write_lock.children.extend(children);
    }

    new_node
}

pub fn absorb<T: GraphNodeData, U: GraphNodeData>(recipient: Graph<T>, donor: Graph<U>) {
    log::trace!("In absorb");

    let recipient_child = {
        recipient
            .read()
            .unwrap()
            .children
            .iter()
            .find(|item| {
                item.read().unwrap().hash == donor.read().unwrap().hash
            })
            .cloned()
    };

    if let Some(recipient_child) = recipient_child {
        log::trace!("Donor and recipient node have the same hash");

        if graph_hash(recipient_child.clone()) != graph_hash(donor.clone()) {
            log::trace!("Donor and recipient child have differing subgraph hashes");
            let donor_children = donor.read().unwrap().children.clone();

            for donor_child in donor_children.iter() {
                absorb(recipient_child.clone(), donor_child.clone());
            }
        }
    } else {
        log::trace!("Donor and recipient subgraphs incompatible. Adopting donor node...");

        let copied = deep_copy::<T, U>(donor, vec![recipient.clone()]);
        recipient.write().unwrap().children.push(copied.clone());
    }
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
        let node_id = node.read().unwrap().id.clone();
        let node_hash = node.read().unwrap().hash.clone();

        if let Some(first_occurrence) = visited.get(&node_hash) {
            log::info!("Detected cycle");

            for parent in node.read().unwrap().parents.iter() {
                let mut write_lock = parent.write().unwrap();
                write_lock.children.retain(|child| child.read().unwrap().id != node_id);
                write_lock.children.push(first_occurrence.clone());
            }

            let children = node.write().unwrap().children.drain(..).collect::<Vec<_>>();
            for child in children {
                let mut write_lock = child.write().unwrap();
                write_lock.parents.retain(|parent| parent.read().unwrap().id != node_id);
                write_lock.parents.push(first_occurrence.clone());

                first_occurrence.write().unwrap().children.push(child.clone());
            }
        } else {
            visited.insert(node_hash.clone(), Arc::clone(&node));

            let children: Vec<Graph<T>>;
            {
                let read_lock = node.read().unwrap();
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

pub fn prune<T: GraphNodeData>(graph: Graph<T>) {
    log::trace!("In prune");

    fn is_twin<T: GraphNodeData>(a: Graph<T>, b: Graph<T>) -> bool {
        let a_rl = a.read().unwrap();
        let b_rl = b.read().unwrap();
        a_rl.id != b_rl.id && a_rl.hash == b_rl.hash
    }

    bft(Arc::clone(&graph), &mut |parent: Graph<T>| {
        loop {
            let children: Vec<Graph<T>>;
            {
                let read_lock = parent.read().unwrap();
                children = read_lock.children.clone();
            }

            let maybe_twins: Option<(Graph<T>, Graph<T>)> = children
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
            } else {
                break;
            }
        }

        true
    });
}

pub async fn interpret(graph: Graph<BasisNode>, output_tree: Graph<XmlNode>) {
    log::trace!("In interpret");

    let mut nodes: Vec<Graph<BasisNode>> = Vec::new();

    bft(Arc::clone(&graph), &mut |node: Graph<BasisNode>| {
        nodes.push(node.clone());
        true
    });

    let semaphore = Arc::new(Semaphore::new(constants::MAX_CONCURRENCY));
    let mut handles = vec![];

    for node in nodes.iter() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        handles.push(
            task::spawn(
                analyze(
                    Arc::clone(node),
                    Arc::clone(&graph),
                    Arc::clone(&output_tree),
                    permit
                )));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

pub fn get_lineage(tree_node: Graph<XmlNode>) -> VecDeque<String> {
    let mut lineage = VecDeque::new();
    let mut current_node = tree_node;

    loop {
        lineage.push_front(read_lock!(current_node).hash.clone());
        
        let parents = read_lock!(current_node).parents.clone();

        if let Some(parent) = parents.first() {
            current_node = Arc::clone(parent);
        } else {
            break;
        }
    }

    lineage
}

pub fn apply_lineage(basis_graph: Graph<BasisNode>, mut lineage: VecDeque<String>) -> Graph<BasisNode> {
    let binding = read_lock!(basis_graph);
    let mut current_node = binding.children.first().expect("Expected basis graph to contain a child").clone();

    while let Some(hash) = lineage.pop_front() {
        let node = read_lock!(current_node)
            .children
            .iter()
            .find(|child| read_lock!(child).hash == hash)
            .cloned();

        if let Some(node) = node {
            current_node = Arc::clone(&node);
        }
    }

    current_node.clone()
}

pub fn find_homologous_nodes(
    target_node: Graph<BasisNode>,
    basis_graph: Graph<BasisNode>,
    output_tree: Graph<XmlNode>,
) -> Vec<Graph<XmlNode>> {
    log::trace!("In find_homologous_nodes");

    let mut homologous_nodes: Vec<Graph<XmlNode>> = Vec::new();

    bft(Arc::clone(&output_tree), &mut |output_node: Graph<XmlNode>| {
        let lineage = get_lineage(output_node.clone());
        let basis_node: Graph<BasisNode> = apply_lineage(Arc::clone(&basis_graph), lineage);

        if read_lock!(basis_node).id == read_lock!(target_node).id {
            homologous_nodes.push(output_node);
        }

        true
    });

    homologous_nodes
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

    {
        discard_node.write().unwrap().parents.clear();
    }

    for child in discard_node.read().unwrap().children.iter() {
        let mut write_lock = child.write().unwrap();
        write_lock.parents.retain(|p| p.read().unwrap().id != discard_node.read().unwrap().id);
        write_lock.parents.push(Arc::clone(&keep_node));

        keep_node.write().unwrap().children.push(Arc::clone(child));
    }

    parent.write().unwrap().children.retain(|child| child.read().unwrap().id != discard_node.read().unwrap().id);
}
