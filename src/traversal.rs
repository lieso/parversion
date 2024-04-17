use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

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

pub fn post_order_traversal(node: Rc<Node>, visit: &mu dyn FnMut(&Rc<Node>)) {
    for child in node.children.borrow().iter() {
        post_order_traversal(child.clone(), visit);
    }

    visit(&node);
}

impl Traversal {
    pub fn from_tree(tree: Rc<Node>) -> Self {
        Traversal {
            ancestry_hash: tree.ancestry_hash,
            subtree_hash: tree.subtree_hash,
            complex_types: Vec::new(),
            complex_object: Vec::new(),
            lists: Vec::new(),
            relationships: Vec::(),
        }
    }

    pub fn with_basis(&self, tree: Rc<Node>) -> Self {
        self.basis = Rc.clone(tree);
    }

    pub fn traverse(&self) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(self.tree.clone());

        while let Some(current) = queue.pop_front() {

            let path = current.get_path();

            let basis_node = trees::walk_path(path).expect().unwrap();



            for child in current.children.borrow().iter() {
                queue.push_back(child.clone());
            }
        }
    }

    pub fn harvest() -> Output {
        Output {

        }
    }
}
