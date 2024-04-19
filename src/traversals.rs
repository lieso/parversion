use std::collections::VecDeque;
use std::rc::{Rc};

use crate::models::*;
//use crate::trees;

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



//            let lineage = current.get_lineage();
//
//            if let Some(basis_node) = trees::search_tree_by_lineage(basis_tree.clone(), lineage) {
//
//                if basis_node.is_complex_node() {
//                    self.complex_objects.push(
//                        trees::map_complex_object(basis_node, current.clone())
//                    );
//                } else {
//                    self.primitives.push(
//                        trees::map_primitives(basis_node, current.clone())
//                    );
//                }
//
//            } else {
//                log::warn!("Basis tree does to contain corresponding node to output tree!");
//                continue;
//            }





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
