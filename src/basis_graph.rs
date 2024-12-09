use crate::transformations::{SchemaTransformation};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MutableBasisGraph {
    pub id: String,
    pub name: String,
    pub description: String,
    pub has_recursive: bool,
    pub graph_nodes: HashMap<String, BasisNode>,
    pub graph_networks: HashMap<String, BasisNetwork>,
    pub source_json_schema: String,
    pub transformations: HashMap<(String, String), Vec<SchemaTransformation>>,
    pub json_schema: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImmutableBasisGraph {
    pub id: Arc<String>,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub has_recursive: Arc<bool>,
    pub graph_nodes: HashMap<String, BasisNode>,
    pub graph_networks: HashMap<String, BasisNetwork>,
    pub source_json_schema: String,
    pub transformations: HashMap<(String, String), Vec<SchemaTransformation>>,
    pub json_schema: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DefaultBasisGraph {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub has_recursive: Option<bool>,
    pub graph_nodes: HashMap<String, BasisNode>,
    pub graph_networks: HashMap<String, BasisNetwork>,
    pub default_json_schema: Option<String>,
    pub transformations: HashMap<(String, String), Vec<SchemaTransformation>>,
    pub normal_json_schema: Option<String>,
}

impl Default for DefaultBasisGraph {
    fn default() -> Self {
        DefaultBasisGraph {
            id: Uuid::new_v4().to_string(),
            name:  None,
            description: None,
            has_recursive: None,
            graph_nodes: HashMap::new(),
            default_json_schema = None,
            transformations: HashMap::new(),
            json_schema: None,
        }
    }
}

impl BasisGraph {
    pub fn perform_network_analysis(self, json_tree: Graph<JsonNode>) {

    }

    pub async fn perform_node_analysis(self, input_graph: Graph<XmlNode>) {

        bft(Arc::clone(&input_graph), &mut |node: Graph<XmlNode>| {
            let lineage = read_lock!(node).lineage.clone();

            if self.nodes.contains_key(&lineage) {
                log::info!("Basis graph already contains input node");
                return true;
            }

            let xml_transformations = 

        });

    }

    pub fn apply_node_transformations(self, output_tree: Graph<XmlNode>, json_tree: Graph<JsonNode>) {
        let unique_paths = HashMap<String, (SchemaPath, Value)> = HashMap::new();

        let mut related_data: Value;
        let mut data: Value;
        
        let mut current_path = SchemaPath::Default();

        fn recurse(

        ) {
            Arc::clone(&output_tree),
            &self,
            &mut current_path,
            &mut data,
            &mut related_data
        }


        let schema = build_schema_from_paths(unique_paths.values,);
        self.source_json_schema = schema;
    }

    pub fn apply_network_transformations(self, json_tree: Graph<JsonNode>) -> Graph<JsonNde> {

    }

    pub fn 
}


















lazy_static! {
    pub static ref BASIS_GRAPHS: Vec<BasisGraph> = vec![
        BasisGraph {
            id: String::from("7bba3bdf-3343-4f71-a0c9-c24a076dc7e8"),
            name: String::from("content_aggregator"),
            description: String::from("Content aggregators are web platforms that curate and compile information from various sources, presenting it in a single, convenient location for users to easily access and explore. These platforms do not typically produce original content themselves but instead collate articles, news stories, blog posts, and other digital media from across the internet. Examples of popular content aggregators include Reddit, where users submit and vote on links, creating dynamic discussions and community-driven content relevance; Hacker News, which features a constantly updated mix of significant tech and startup industry news curated by user submissions; and Google Search Results, which aggregate webpages, images, videos, and other types of content based on user queries, offering a broad spectrum of the most relevant and authoritative sources available online. Content aggregators serve as valuable tools for staying informed by allowing users to discover content aligned with their interests, preferences, or professional needs efficiently."),
            has_recursive: false,
            json_schema: String::from(r#"
            {
               "$schema": "http://json-schema.org/draft-07/schema#",
               "title": "Content Aggregator",
               "type": "object",
               "properties": {
                 "entries": {
                   "type": "array",
                   "description": "A list of content entries aggregated by the application.",
                   "items": {
                     "type": "object",
                     "properties": {
                       "title": {
                         "type": "string",
                         "description": "The main title of each entry, typically displayed prominently."
                       },
                       "url": {
                         "type": "string",
                         "description": "The URL directing to the original content."
                       },
                       "score": {
                         "type": "string",
                         "description": "The popularity score of the entry, reflecting its user engagement."
                       },
                       "submitted": {
                         "type": "string",
                         "description": "The timestamp indicating when the entry was submitted."
                       }
                     },
                     "required": ["title", "url", "submitted"]
                   }
                 }
               },
               "required": ["entries"]
             }
            "#),
            transformations: serde_json::from_str(r#"
            {
                "source,target": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ],
                "source2,target2": [
                    {"id": 3, "name": "Charlie"},
                    {"id": 4, "name": "David"}
                ]
            }
            "#).expect("Error parsing transformations"),
            nodes: serde_json::from_str(r#"
            []
            "#).expect("Error parsing nodes"),
        },
    ];
}
