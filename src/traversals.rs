use std::collections::{HashMap, VecDeque};
use std::rc::{Rc};

use crate::models::*;

pub fn map_primitives(basis_tree: Rc<Node>, output_tree: Rc<Node>) -> HashMap<String, String> {
    unimplemented!()
}

pub fn map_complex_object(basis_tree: Rc<Node>, output_tree: Rc<Node>) -> ComplexObject {
    unimplemented!()
}

pub fn search_tree_by_lineage(basis_tree: Rc<Node>, lineage: VecDeque<String>) -> Option<Rc<Node>> {
    unimplemented!()
}

pub fn get_lineage(node: Rc<Node>) -> VecDeque<String> {
    let mut lineage = VecDeque::new();
    lineage.push_back(node.hash.clone());

    let mut current_parent = node.parent.borrow().clone();

    while let Some(parent) = current_parent {
        lineage.push_front(parent.hash.clone());

        current_parent = {
            let node_ref = parent.parent.borrow();
            node_ref.as_ref().map(|node| node.clone())
        };
    }

    lineage
}

pub fn bfs(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    let mut queue = VecDeque::new();
    queue.push_back(node.clone());

    while let Some(current) = queue.pop_front() {
        visit(&current);

        for child in current.children.borrow().iter() {
            queue.push_back(child.clone());
        }
    }
}

pub fn dfs(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    visit(&node);

    for child in node.children.borrow().iter() {
        dfs(child.clone(), visit);
    }
}

pub fn post_order_traversal(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    for child in node.children.borrow().iter() {
        post_order_traversal(child.clone(), visit);
    }

    visit(&node);
}

impl Traversal {
    pub fn from_tree(tree: Rc<Node>) -> Self {
        Traversal {
            output_tree: tree,
            basis_tree: None,
            primitives: Vec::new(),
            complex_types: Vec::new(),
            complex_objects: Vec::new(),
            lists: Vec::new(),
            relationships: Vec::new(),
        }
    }

    pub fn with_basis(mut self, tree: Rc<Node>) -> Self {
        self.basis_tree = Some(Rc::clone(&tree));
        
        self
    }

    pub fn traverse(mut self) -> Result<Self, Errors> {

        let basis_tree = self.basis_tree.clone().unwrap();

        let mut bfs: VecDeque<Rc<Node>> = VecDeque::new();
        bfs.push_back(Rc::clone(&self.output_tree));

        while let Some(current) = bfs.pop_front() {



            let lineage = get_lineage(Rc::clone(&current));

            if let Some(basis_node) = search_tree_by_lineage(basis_tree.clone(), lineage) {

                if basis_node.complex_type_name.borrow().is_some() {
                    self.complex_objects.push(
                        map_complex_object(basis_node, current.clone())
                    );
                }

            } else {
                log::warn!("Basis tree does to contain corresponding node to output tree!");
                continue;
            }





            for child in current.children.borrow().iter() {
                bfs.push_back(child.clone());
            }
        }

        Ok(self)
    }

    // to json, to xml, etc.
    pub fn harvest(self) -> Result<Output, Errors> {
        unimplemented!()
    }
}
