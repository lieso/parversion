mod ast;
mod classification;
mod basis_network;
mod basis_node;
mod bloom_filter;
#[cfg(feature = "caching")]
mod cache;
mod config;
mod context;
mod context_group;
mod data_node;
mod document;
mod document_format;
mod document_node;
mod entrypoint;
mod environment;
mod function;
mod function_analysis;
mod graph_node;
mod hash;
mod id;
mod json_node;
mod lineage;
mod llm;
mod macros;
mod meta_context;
mod metadata;
mod mutation;
mod network_analysis;
mod node_analysis;
mod normalization;
mod operation;
mod options;
mod organization;
mod package;
mod path;
mod path_segment;
mod prelude;
mod profile;
mod provider;
mod query;
mod schema;
mod schema_context;
mod schema_node;
mod timestamp;
mod transformation;
mod translation;
mod types;
#[allow(dead_code)]
mod utility;
mod execution_context;
mod network_relationship;
mod basis_graph;
mod xpath;
mod traversal;

use crate::entrypoint::run;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        println!("Error occurred: {:?}", e);
        std::process::exit(1);
    }
    std::process::exit(0);
}
