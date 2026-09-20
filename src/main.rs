mod basis_field;
mod basis_graph;
mod basis_group;
mod basis_network;
mod basis_node;
mod classification;
mod config;
mod context;
mod data_node;
mod document;
mod document_format;
mod document_node;
mod entrypoint;
mod environment;
mod execution_context;
mod field_analysis;
mod graph_analysis;
mod graph_node;
mod group_analysis;
mod hash;
mod id;
mod json_node;
mod lineage;
mod llm;
mod macros;
mod meta_context;
mod metadata;
mod network_analysis;
mod node_analysis;
mod normal_context;
mod normal_meta_context;
mod normalization;
mod normalization_context;
mod options;
mod package;
mod prelude;
mod prompt_registry;
mod provider;
mod reasoner;
mod reports;
mod transformation;
mod translation;
mod translation_context;
mod translation_network;
mod translation_node;
mod types;
#[allow(dead_code)]
mod utility;
mod xpath;

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
        eprintln!("Error occurred: {:?}", e);
        log::error!("Fatal error: {:?}", e);
        std::process::exit(1);
    }
    std::process::exit(0);
}
