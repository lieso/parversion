
// pruning and cyclizing not required
// just need lineage


// get, apply document transformations
// convert document to data nodes
// obtain unique nodes
// compare against basis nodes
// if basis node found, apply transformations to get json node
// else: analyse data node to get basis node and then get json node

pub struct Analysis {
    pub data_nodes: HashMap<ID, DataNode>,
    pub graph: GraphNode,
    pub basis_graph: BasisGraph,
}

impl Analysis {
    pub fn from_document(document: Document) -> Self {



        let document_transformations = document.get_transformations();

        document.apply_transformations(document_transformations);





        let document_root = docment.get_root_node();

        let data_nodes: HashMap<ID, DataNode> = HashMap::from(
            vec![
                document_root.0.id.to_string(),
                document_root.0.clone()
            ]
        );




        fn recurse(
            document_data: (DataNode, Vec<DocumentNode>),
            parents: Vec<Rc<GraphNode>>
        ) {
            let data_node = document_data.0;

            data_nodes.insert(data_node.id.to_string(), data_node.clone());

            let mut graph_node = GraphNode {
                id: ID::new(),
                parents,
                children: Vec::new(),
                origin_node_id: document_data.0.id.to_string()
            };

            let children: document_data.1.iter().map(|child| {
                recurse(
                    Document::document_to_data(child, Some(nodes.0)),
                    Rc::new(graph_node),
                )
            });

            graph_node.children.extend(children);

            graph_node
        }





        Analysis {
            document_transformations,
        }
    }
}
