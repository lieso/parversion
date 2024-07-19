use std::rc::{Rc};
use dot::{GraphWalk, Labeller};
use std::fs::File;
use std::collections::HashSet;

use super::{Node};

impl Node {
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

impl<'a> Labeller<'a, Rc<Node>, (Rc<Node>, Rc<Node>)> for Node {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("tree").unwrap()
    }

    fn node_id(&'a self, node: &Rc<Node>) -> dot::Id<'a> {
        let id = "node_".to_owned() + &node.id.replace("-", "");
        dot::Id::new(id.clone()).unwrap()
    }

    fn node_label(&'a self, node: &Rc<Node>) -> dot::LabelText<'a> {
        //let label = node.id.clone().chars().take(7).collect::<String>();
        //let label = node.hash.clone().chars().take(7).collect::<String>();
        //let label = node.xml.get_all_tags().join(",");
        let label = node.xml.to_string().chars().take(10).collect::<String>();
        dot::LabelText::label(label)
    }
}

impl<'a> GraphWalk<'a, Rc<Node>, (Rc<Node>, Rc<Node>)> for Node {
    fn nodes(&self) -> dot::Nodes<Rc<Node>> {
        let mut nodes = vec![];
        let self_rc = Rc::new(self.clone());
        self.collect_nodes(&self_rc, &mut nodes, &mut HashSet::new());
        nodes.into()
    }

    fn edges(&self) -> dot::Edges<(Rc<Node>, Rc<Node>)> {
        let mut edges = vec![];
        let self_rc = Rc::new(self.clone());
        self.collect_edges(&self_rc, &mut edges, &mut HashSet::new());
        edges.into()
    }

    fn source(&self, edge: &(Rc<Node>, Rc<Node>)) -> Rc<Node> {
        edge.0.clone()
    }

    fn target(&self, edge: &(Rc<Node>, Rc<Node>)) -> Rc<Node> {
        edge.1.clone()
    }
}

impl Node {
    fn collect_nodes(
        &self,
        node: &Rc<Node>,
        nodes: &mut Vec<Rc<Node>>,
        visited: &mut HashSet<String>,
    ) {
        if visited.insert(node.id.clone()) {
            nodes.push(node.clone());
            for child in node.children.borrow().iter() {
                self.collect_nodes(child, nodes, visited);
            }
        }
    }

    fn collect_edges(
        &self,
        node: &Rc<Node>,
        edges: &mut Vec<(Rc<Node>, Rc<Node>)>,
        visited: &mut HashSet<String>,
    ) {
        if visited.insert(node.id.clone()) {
            for child in node.children.borrow().iter() {
                edges.push((node.clone(), child.clone()));
                self.collect_edges(child, edges, visited);
            }

            if let Some(parent) = node.parent.borrow().as_ref() {
                edges.push((node.clone(), parent.clone()));
            }
        }
    }
}
