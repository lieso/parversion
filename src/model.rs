use lazy_static::lazy_static;
use std::collections::HashMap;

use crate::id::{ID};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub id: ID,

    /// The JSON schema defining the structure of the data model
    pub json_schema: String,

    /// The name of the data model
    pub name: String,

    /// A detailed description explaining the applications and usage of the data model
    pub description: String,

    /// Tags for categorizing or labeling the data model, useful for searching and organizing
    pub tags: Vec<String>,

    /// Metadata that provides additional information, e.g., author, creation date
    pub metadata: HashMap<String, String>,

    /// A sample instance of data for demonstration or testing purposes
    pub example_data: Option<String>,
}

lazy_static! {
    pub static ref MODELS: Vec<Model> = vec![
        Model {
            id: ID::from_str("7bba3bdf-3343-4f71-a0c9-c24a076dc7e8"),
            name: String::from("content_aggregator"),
            description: String::from("Content aggregators are web platforms that curate and compile information from various sources, presenting it in a single, convenient location for users to easily access and explore. These platforms do not typically produce original content themselves but instead collate articles, news stories, blog posts, and other digital media from across the internet. Examples of popular content aggregators include Reddit, where users submit and vote on links, creating dynamic discussions and community-driven content relevance; Hacker News, which features a constantly updated mix of significant tech and startup industry news curated by user submissions; and Google Search Results, which aggregate webpages, images, videos, and other types of content based on user queries, offering a broad spectrum of the most relevant and authoritative sources available online. Content aggregators serve as valuable tools for staying informed by allowing users to discover content aligned with their interests, preferences, or professional needs efficiently."),
            json_schema: json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The URL of the source content" },
                    "timestamp": { "type": "string", "format": "date-time", "description": "The date and time the content was aggregated" },
                    "content": { "type": "string", "description": "The aggregated content details" },
                    "author": { "type": "string", "description": "The author of the content" },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Tags associated with the aggregated content"
                    }
                },
                "required": ["source", "timestamp", "content"]
            }).to_string(),
            tags: vec!["content", "aggregation", "news", "information"].into_iter().map(String::from).collect(),
            metadata: serde_json::from_str::<HashMap<String, String>>(r#"
                {
                    "author": "Jane Doe",
                    "created": "2023-10-01",
                    "version": "1.0"
                }
            "#).expect("Error parsing metadata"),
            example_data: Some(r#"
                {
                    "source": "https://example.com",
                    "timestamp": "2023-10-05T14:48:00Z",
                    "content": "This is an example of aggregated content.",
                    "author" "Anonymous",
                    "tags": ["example", "test"]
                }"#.to_string()
            ),
        }
    ];
}
