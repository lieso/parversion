use std::sync::{Arc, RwLock};
use dot::{GraphWalk, Labeller};
use std::collections::HashSet;
use std::fs::File;

use super::{GraphNode, Graph, GraphNodeData};

impl<T: GraphNodeData> GraphNode<T> {
    pub fn debug_visualize(&self, label: &str) {
        let dot_path = format!("./debug/{}.dot", label);
        let png_path = format!("./debug/{}.png", label);
        let mut file = File::create(dot_path.clone()).expect("Unable to create file");
        dot::render(self, &mut file).expect("Unable to render dot file");

        std::process::Command::new("dot")
            .args(&["-Tpng", &dot_path, "-o", &png_path])
            .output()
            .expect("Failed to execute dot command");
    }
}

impl<'a, T: GraphNodeData> Labeller<'a, Graph<T>, (Graph<T>, Graph<T>)> for GraphNode<T> {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("tree").unwrap()
    }

    fn node_id(&'a self, node: &Graph<T>) -> dot::Id<'a> {
        let id = "node_".to_owned() + &node.read().unwrap().id.replace("-", "");
        dot::Id::new(id.clone()).unwrap()
    }

    fn node_label(&'a self, node: &Graph<T>) -> dot::LabelText<'a> {
        let label = node.read().unwrap().hash.chars().take(20).collect::<String>();
        dot::LabelText::label(label)
    }
}

impl<'a, T: GraphNodeData> GraphWalk<'a, Graph<T>, (Graph<T>, Graph<T>)> for GraphNode<T> {
    fn nodes(&self) -> dot::Nodes<Graph<T>> {
        let mut nodes = vec![];
        let self_arc = Arc::new(RwLock::new((*self).clone()));
        self.collect_nodes(&self_arc, &mut nodes, &mut HashSet::new());
        nodes.into()
    }

    fn edges(&self) -> dot::Edges<(Graph<T>, Graph<T>)> {
        let mut edges = vec![];
        let self_arc = Arc::new(RwLock::new((*self).clone()));
        self.collect_edges(&self_arc, &mut edges, &mut HashSet::new());
        edges.into()
    }

    fn source(&self, edge: &(Graph<T>, Graph<T>)) -> Graph<T> {
        edge.0.clone()
    }

    fn target(&self, edge: &(Graph<T>, Graph<T>)) -> Graph<T> {
        edge.1.clone()
    }
}

impl<T: GraphNodeData> GraphNode<T> {
    fn collect_nodes(
        &self,
        node: &Graph<T>,
        nodes: &mut Vec<Graph<T>>,
        visited: &mut HashSet<String>,
    ) {
        if visited.insert(node.read().unwrap().id.clone()) {
            nodes.push(node.clone());
            for child in node.read().unwrap().children.iter() {
                self.collect_nodes(child, nodes, visited);
            }
        }
    }

    fn collect_edges(
        &self,
        node: &Graph<T>,
        edges: &mut Vec<(Graph<T>, Graph<T>)>,
        visited: &mut HashSet<String>,
    ) {
        if visited.insert(node.read().unwrap().id.clone()) {
            for child in node.read().unwrap().children.iter() {
                edges.push((node.clone(), child.clone()));
                self.collect_edges(child, edges, visited);
            }

            for parent in node.read().unwrap().parents.iter() {
                edges.push((node.clone(), parent.clone()));
            }
        }
    }
}
