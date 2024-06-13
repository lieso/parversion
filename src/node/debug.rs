use std::rc::{Rc};
use std::fs::OpenOptions;
use std::io::Write;
use dot::{GraphWalk, Labeller};
use std::fs::File;

use super::{Node};
use crate::node::traversal;

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

    pub fn log_tree(&self, title: &str) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("./debug/trees")
            .expect("Could not open file");

        let divider = std::iter::repeat("*").take(100).collect::<String>();
        let text = format!(
            "\n\n{} {}\n",
            divider,
            title
            );

        writeln!(file, "{}", text).expect("Could not write to file");

        let mut node_count = 0;

        traversal::bfs(self.clone().into(), &mut |node: &Rc<Node>| {
            node_count = node_count + 1;

            let divider = std::iter::repeat("-").take(50).collect::<String>();
            let text = format!(
                "\nID: {}\nHASH: {}\nXML: {}\nSUBTREE HASH: {}\nANCESTOR HASH: {}\nCOMPLEX TYPE NAME: {:?}\n",
                node.id,
                node.hash,
                node.xml,
                node.subtree_hash(),
                node.ancestry_hash(),
                node.complex_type_name
                );

            let mut node_data_text = String::from("");

            for d in node.data.borrow().iter() {
                node_data_text = node_data_text + format!(r##"
                    name: {},
                    value: {:?}
                "##, d.name, d.value).as_str();
            }

            let text = format!("\n{}{}{}{}\n", divider, text, node_data_text, divider);

            writeln!(file, "{}", text).expect("Could not write to file");
        });

        writeln!(file, "node count: {}", node_count).expect("Could not write to file");
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
        let label = node.hash.clone().chars().take(7).collect::<String>();
        dot::LabelText::label(label)
    }
}

impl<'a> GraphWalk<'a, Rc<Node>, (Rc<Node>, Rc<Node>)> for Node {
    fn nodes(&self) -> dot::Nodes<Rc<Node>> {
        let mut nodes = vec![];
        let self_rc = Rc::new(self.clone());
        self.collect_nodes(&self_rc, &mut nodes);
        nodes.into()
    }

    fn edges(&self) -> dot::Edges<(Rc<Node>, Rc<Node>)> {
        let mut edges = vec![];
        let self_rc = Rc::new(self.clone());
        self.collect_edges(&self_rc, &mut edges);
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
    fn collect_nodes(&self, node: &Rc<Node>, nodes: &mut Vec<Rc<Node>>) {
        nodes.push(node.clone());
        for child in node.children.borrow().iter() {
            self.collect_nodes(child, nodes);
        }
    }

    fn collect_edges(&self, node: &Rc<Node>, edges: &mut Vec<(Rc<Node>, Rc<Node>)>) {
        for child in node.children.borrow().iter() {
            edges.push((node.clone(), child.clone()));
            self.collect_edges(child, edges);
        }
    }
}
