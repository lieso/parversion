use std::rc::{Rc};
use std::collections::{VecDeque, HashSet};

use super::Node;

pub fn bfs_graph(tree: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    let mut queue = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();

    queue.push_back(tree);

    while let Some(node) = queue.pop_front() {
        visit(&node);

        for child in node.children.borrow().iter() {
            if visited.contains(&child.id) {

            } else {
                queue.push_back(Rc::clone(child));
                visited.insert(child.id.clone());
            }
        }
    }
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

pub fn post_order_traversal(node: Rc<Node>, visit: &mut dyn FnMut(&Rc<Node>)) {
    for child in node.children.borrow().iter() {
        post_order_traversal(child.clone(), visit);
    }

    visit(&node);
}
