
pub struct Json {}

impl Json {
    fn from_normalized_graph(
        meta_context: Arc<RwLock<MetaContext>>,
    ) -> Result<String, Errors> {
        log::trace!("In from_normalized_graph_json");

        let graph_root = read_lock!(meta_context).normal_graph_root.clone().unwrap();

        let mut result: Map<String, Value> = Map::new();

        fn recurse(
            meta_context: Arc<RwLock<MetaContext>>,
            graph_node: Arc<RwLock<GraphNode>>,
            result: &mut Map<String, Value>,
        ) {
            let contexts = {
                let lock = read_lock!(meta_context);
                lock.normal_contexts.clone().unwrap()
            };

            let context = contexts.get(&read_lock!(graph_node).id).unwrap();
            let network_name = &context.network_name;
            let network_description = &context.network_description;
            let data_node = &context.data_node;
            let json_nodes: Vec<JsonNode> = data_node.to_json_nodes();

            for json_node in json_nodes {
                let json = json_node.json;
                let value = json!(json.value.trim().to_string());
                result.insert(json.key, value);
            }

            for child in &read_lock!(graph_node).children {
                let child_context = contexts.get(&read_lock!(child).id).unwrap();

                if let Some(child_network_name) = &child_context.network_name {
                    log::debug!("child_network_name: {}", child_network_name);

                    let mut inner_result: Map<String, Value> = Map::new();

                    recurse(
                        Arc::clone(&meta_context),
                        Arc::clone(&child),
                        &mut inner_result
                    );

                    let inner_result_value = Value::Object(inner_result.clone());

                    if let Some(existing_object) = result.get_mut(child_network_name) {
                        if let Value::Array(ref mut arr) = existing_object {
                            arr.push(inner_result_value.clone());
                        } else {
                            *existing_object = json!(vec![
                                existing_object.clone(),
                                inner_result_value.clone()
                            ]);
                        }
                    } else {
                        result.insert(child_network_name.clone(), inner_result_value);
                    }

                } else {
                    recurse(
                        Arc::clone(&meta_context),
                        Arc::clone(&child),
                        result
                    );
                }
            }
        }

        recurse(
            Arc::clone(&meta_context),
            Arc::clone(&graph_root),
            &mut result,
        );

        let data = serde_json::to_string_pretty(&result).expect("Could not make a JSON string");

        Ok(data)
    }
}
