mod basis_network;
mod basis_node;
mod basis_graph;
#[cfg(feature = "caching")]
mod cache;
mod config;
mod data_node;
mod document;
mod document_format;
mod document_node;
mod environment;
mod graph_node;
mod hash;
mod id;
mod lineage;
mod macros;
mod normalization;
mod organization;
mod profile;
mod provider;
mod transformation;
mod translation;
mod types;
mod prelude;
#[allow(dead_code)]
mod utility;
mod json_node;
mod context;
mod context_group;
mod llm;
mod meta_context;
mod schema;
mod network_analysis;
mod node_analysis;
mod schema_node;
mod schema_context;
mod path;
mod mutation;
mod function;
mod ast;
mod package;
mod metadata;
mod function_analysis;
mod entrypoint;

use crate::entrypoint::run;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        println!("Error occurred: {:?}", e);
        std::process::exit(1);
    }
    std::process::exit(0);
}
