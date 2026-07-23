mod classification;
mod basis_network;
mod basis_node;
#[cfg(feature = "caching")]
mod cache;
mod config;
mod context;
mod data_node;
mod document;
mod document_format;
mod document_node;
mod entrypoint;
mod environment;
mod graph_node;
mod hash;
mod id;
mod json_node;
mod lineage;
mod llm;
mod macros;
mod normalization_context;
mod metadata;
mod mutation;
mod network_analysis;
mod node_analysis;
mod operation;
mod options;
mod normalization;
mod package;
mod prelude;
mod provider;
mod query;
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
mod normal_context;
mod basis_group;
mod reports;
mod basis_field;
mod translation_context;
mod translation_node;
mod meta_context;
mod translation_network;
mod reasoner;
mod prompt_registry;
mod field_analysis;
mod group_analysis;
mod graph_analysis;

use crate::entrypoint::run;

fn build_runtime() -> tokio::runtime::Runtime {
    let mut builder = if std::env::var("SINGLE_THREAD").is_ok() {
        tokio::runtime::Builder::new_current_thread()
    } else {
        tokio::runtime::Builder::new_multi_thread()
    };
    builder.enable_all().build().unwrap()
}

fn main() {
    let runtime = build_runtime();
    if let Err(e) = runtime.block_on(run()) {
        println!("Error occurred: {:?}", e);
        std::process::exit(1);
    }
    std::process::exit(0);
}
