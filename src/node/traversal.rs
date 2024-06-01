use super::Node;

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
