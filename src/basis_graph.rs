use crate::transformations::{SchemaTransformation};
use crate::id::{ID};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasisGraph {
    pub id: ID,
    pub name: String,
    pub description: String,
    pub json_schema: String,
    pub nodes: HashMap<ID, BasisNode>,
    pub networks: HashMap<ID, BasisNetwork>,
}

pub async fn classify_text(
    text: String
) -> Option<BasisGraph> {
    unimplemented!()
}

pub async fn create_basis_graph(
    text: String
) -> BasisGraph {
    unimplemented!()
}

pub async fn classify_or_create_basis_graph(
    text: String
) -> BasisGraph {
    unimplemented!()
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
            nodes: serde_json::from_str(r#"
            []
            "#).expect("Error parsing nodes"),
            networks: serde_json::from_str(r#"
            []
            "#).expect("Error parsing networks"),
        },
    ];
}
