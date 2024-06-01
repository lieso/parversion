impl Node {
    pub fn should_update_node_data(&self) -> bool {
        log::trace!("In should_update_node_data");

        !self.is_structural
    }

    pub fn should_interpret_node_data(&self) -> bool {
        log::trace!("In should_interpret_node_data");
        
        // Do not give a node a type if:
        // * It's a leaf node - whoops yes we do. e.g. title tag in head is a simple text node, but will need 'page_title' complex type
        // * It and all children are structural nodes
        // * It has no data AND children neither have data or a complex type

        //let is_leaf = self.children.borrow().is_empty();
        //log::debug!("is_leaf: {}", is_leaf);

        //if is_leaf {
        //    return false;
        //}

        if self.data.borrow().is_empty() {


            return false;
        }

        let is_structural = self.children.borrow().iter().fold(
            self.is_structural,
            |acc, item| {
                acc && item.is_structural
            }
        );
        log::debug!("is_structural: {}", is_structural);

        if is_structural {
            return false;
        }

        true
    }

    pub fn should_propagate_node_interpretation(&self) -> Option<String> {
        log::trace!("In should_propagate_node_interpretation");

        // We should propagate descendant complex type to parent if:
        // Node only has one non-structural child
        // TODO: what if node and all children except one are structural, and structural node is leaf node?

        let non_structural_count: u16 = self.children.borrow().iter().fold(
            0 as u16,
            |acc, item| {
                acc + !item.is_structural as u16
            }
        );

        if self.is_structural && non_structural_count == 1 {
            log::info!("Node is structural and has exactly one non-structural child");

            let sole_non_structural_node: Rc<Node> = self.children.borrow().iter().find(|item| {
                !item.is_structural
            }).unwrap().clone();

            let complex_type_name = sole_non_structural_node.complex_type_name.borrow().clone().unwrap();

            return Some(complex_type_name);
        }

        None
    }

    pub fn should_classically_update_node_data(&self) -> Option<Vec<NodeData>> {
        log::trace!("In should_classically_update_node_data");

        // * We don't need to consult an LLM to interpret text nodes

        if self.hash == TEXT_NODE_HASH {

            let is_js = self.parent.borrow().clone().unwrap().xml.is_script_element();

            let node_data = NodeData {
                attribute: None,
                name: String::from("text"),
                regex: String::from("^.*$"),
                value: None,
                is_url: false,
                is_id: false,
                is_decorative: false,
                is_js: is_js,
            };

            return Some(vec![node_data]);
        }

        None
    }

    pub async fn update_node_data(&self, db: &Db) -> bool {
        log::trace!("In update_node_data");

        if !self.should_update_node_data() {
            log::info!("Not updating this node");
            *self.data.borrow_mut() = Vec::new();
            return false;
        }

        if let Some(classical_interpretation) = self.should_classically_update_node_data() {
            log::info!("Node interpreted classically");
            *self.data.borrow_mut() = classical_interpretation;
            return false;
        }

        if let Some(node_data) = get_node_data(&db, &self.xml.to_string()).expect("Could not get node data from database") {
            log::info!("Cache hit!");
            *self.data.borrow_mut() = node_data.clone();
            return false;
        } else {
            log::info!("Cache miss!");

            let llm_node_data: Vec<NodeData> = llm::generate_node_data(self.xml.to_string()).await.expect("LLM unable to generate node data");

            if llm_node_data.len() == 0 {
                log::warn!("Node has been interpreted to have zero data entries. I guess this is now a structural node?");
            }

            *self.data.borrow_mut() = llm_node_data.clone();

            store_node_data(&db, &self.xml.to_string(), llm_node_data.clone()).expect("Unable to persist node data to database");
        }

        true
    }

    pub fn update_node_data_values(&self) {
        let mut data = self.data.borrow_mut();

        log::info!("Node has {} entries", data.len());

        for item in data.iter_mut() {
            if let Some(node_data_value) = item.select(self.xml.clone()) {
                log::trace!("Node data selection success: {}", node_data_value.text);
                item.value = Some(node_data_value);
            } else {
                log::warn!("Node could not obtain data from its own xml!");
                item.value = None;
            }
        }
    }

    pub async fn interpret_node_data(&self, db: &Db) -> bool {
        log::trace!("In interpret_node_data");

        assert!(!self.xml.is_empty());

        //if let Some(propagated_complex_type) = self.should_propagate_node_interpretation() {
        //    log::info!("Propagating node interpretation");
        //    *self.complex_type_name.borrow_mut() = Some(propagated_complex_type);
        //    return false;
        //}

        //if !self.should_interpret_node_data() {
        //    log::info!("Not interpreting this node");
        //    *self.complex_type_name.borrow_mut() = None.into();
        //    return false;
        //}

        log::info!("Consulting LLM for node interpretation...");

        // TODO: had to change from subtree_hash to ancestry_hash, but I don't think this is right either
        let ancestry_hash = &self.ancestry_hash();

        if let Some(complex_type) = get_node_complex_type(&db, ancestry_hash).expect("Could not get node complex type from database") {
            log::info!("Cache hit!");
            *self.complex_type_name.borrow_mut() = Some(complex_type.clone());

            return false;
        } else {
            log::info!("Cache miss!");

            let fields = self.get_node_fields();
            let context = self.get_node_context();




            if fields.is_empty() {
                log::info!("Node has no fields!");
                return false;
            }




            let llm_type_name: String = llm::interpret_node(fields, context).await
                .expect("Could not interpret node");

            *self.complex_type_name.borrow_mut() = Some(llm_type_name.clone()).into();

            store_node_complex_type(&db, ancestry_hash, &llm_type_name).expect("Unable to persist complex type to database");
        }

        true
    }

    pub fn get_node_fields(&self) -> String {

        // TODO: feel this belongs in llm module
        self.children.borrow().iter().fold(
            node_data_to_string(self.data.borrow().clone()),
            |acc, item| {
                if let Some(complex_type_name) = item.complex_type_name.borrow().clone() {
                    format!("{}\n{}: {}", acc, uncapitalize(&complex_type_name), &complex_type_name)
                } else {
                    format!("{}\n{}", acc, node_data_to_string(item.data.borrow().clone()))
                }
            }
        )

    }

    pub fn get_node_context(&self) -> String {
        if self.parent.borrow().is_none() {
            return String::from("These fields are self-contained and appear by themselves without any relevant context.");
        }

        let max_siblings = 4;
        let max_parents = 2;
        let mut context = "\n<-- FIELDS ARE FOUND HERE -->\n".to_string();
        let mut comparison_node_id = self.id.clone();
        let mut parent = self.parent.borrow().clone().unwrap();

        if parent.hash == ROOT_NODE_HASH {
            return String::from("These fields are self-contained and appear by themselves without any relevant context.");
        }

        for _ in 0..max_parents {
            let next_parent = {
                let siblings = parent.children.borrow();
                let position = siblings.iter().position(|node| node.id == comparison_node_id)
                    .expect("Node not found as a child of its own parent");
                let start = position.saturating_sub(max_siblings);
                let end = std::cmp::min(siblings.len(), position + max_siblings);
                let sibling_context = siblings[start..end]
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| i != position - start)
                    .map(|(_, sibling)| sibling.xml.to_string() + "\n")
                    .collect::<String>();

                context = sibling_context + &context;

                log::debug!("parent.xml: {}", parent.xml.to_string());

                context = parent.xml.to_string_with_child_string(context.clone())
                    .expect("Could not embed string inside parent element");

                comparison_node_id = parent.id.clone();

                parent.parent.borrow().clone()
            };

            if let Some(next_parent) = next_parent {
                parent = next_parent;

                if parent.hash == ROOT_NODE_HASH {
                    break;
                }
            } else {
                break;
            }
        }

        context
    }
}

fn node_data_to_string(node_data: Vec<NodeData>) -> String {
    node_data.iter().fold(String::from(""), |acc, item| {
        format!("{}\n{}: {}", acc, item.name, item.value.clone().unwrap().text)
    })
}

fn uncapitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

pub fn store_node_data(db: &Db, key: &str, nodes: Vec<NodeData>) -> Result<(), Box<dyn Error>> {
    let serialized_nodes = serialize(&nodes)?;
    db.insert(key, serialized_nodes)?;
    Ok(())
}

pub fn get_node_data(db: &Db, key: &str) -> Result<Option<Vec<NodeData>>, Box<dyn Error>> {
    match db.get(key)? {
        Some(serialized_nodes) => {
            let nodes_data: Vec<NodeData> = deserialize(&serialized_nodes)?;
            Ok(Some(nodes_data))
        },
        None => Ok(None),
    }
} 

pub fn store_node_complex_type(db: &Db, key: &str, complex_type: &str) -> Result<(), Box<dyn Error>> {
    db.insert(key, complex_type)?;
    Ok(())
}

pub fn get_node_complex_type(db: &Db, key: &str) -> Result<Option<String>, Box<dyn Error>> {
    match db.get(key)? {
        Some(iv) => {
            let complex_type = String::from_utf8(iv.to_vec())?;
            Ok(Some(complex_type))
        },
        None => Ok(None),
    }
} 
