impl Node {
    pub fn ancestry_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(self.hash.clone());

        if let Some(parent) = self.parent.borrow().as_ref() {
            hasher_items.push(
                parent.ancestry_hash()
            );
        }

        hasher_items.sort();
        hasher.update(hasher_items.join(""));

        format!("{:x}", hasher.finalize())
    }

    pub fn subtree_hash(&self) -> String {
        let mut hasher = Sha256::new();

        let mut hasher_items = Vec::new();
        hasher_items.push(self.hash.clone());

        for child in self.children.borrow().iter() {
            hasher_items.push(child.subtree_hash());
        }

        hasher_items.sort();
        hasher.update(hasher_items.join(""));

        format!("{:x}", hasher.finalize())
    }

    pub fn get_lineage(&self) -> VecDeque<String> {
        let mut lineage = VecDeque::new();
        lineage.push_back(self.hash.clone());

        let mut current_parent = self.parent.borrow().clone();

        while let Some(parent) = current_parent {
            lineage.push_front(parent.hash.clone());

            current_parent = {
                let node_ref = parent.parent.borrow();
                node_ref.as_ref().map(|node| node.clone())
            };
        }

        lineage
    }

    pub fn get_depth(&self) -> u16 {
        let mut depth = 0;

        let mut current_parent = self.parent.borrow().clone();

        while let Some(parent) = current_parent {
            depth += 1;

            current_parent = {
                let node_ref = parent.parent.borrow();
                node_ref.as_ref().map(|node| node.clone())
            };
        }

        depth
    }
}
