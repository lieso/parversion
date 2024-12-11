use tokio::runtime::Runtime;
use std::process;
use std::io::{Read};
use std::fs::File;
use std::sync::{Arc};
use std::collections::{HashSet, HashMap};
use serde_json::{Value};

use crate::graph_node::GraphNode;
use crate::graph_node::Graph;
use crate::basis_graph::BasisGraph;
use crate::harvest::{Harvest, HarvestFormats, serialize_harvest};
use crate::graph_node;
use crate::basis_graph::{
    build_basis_graph,
    analyze_graph,
};
use crate::xml_node::{XmlNode};
use crate::error::{Errors};
use crate::harvest::{harvest};
use crate::utility;
use crate::macros::*;
use crate::json_schema::{
    content_to_json_schema,
    get_schema_mapping,
    apply_schema_mapping
};

pub struct Normalization {
    pub basis_graph: BasisGraph,
    pub related_data: OutputData,
    pub normalized_data: OutputData,
}

pub fn normalize_file(
    file_name: String,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_file");
    log::debug!("file_name: {}", file_name);

    let mut text = String::new();

    let mut file = File::open(file_name).unwrap_or_else(|err| {
        eprintln!("Failed to open file: {}", err);
        process::exit(1);
    });

    file.read_to_string(&mut text).unwrap_or_else(|err| {
        eprintln!("Failed to read file: {}", err);
        process::exit(1);
    });

    normalize_text(text, options)
}

pub fn normalize_text(
    text: String,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_text");

    let document = Document::from_string(text, options)?;

    normalize_document(document, options)
}

pub async fn normalize_document(
    document: Document,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_document");

    let organization = organization::organize_document(document, options);

    normalize_organization(organization, options)
}

pub async fn normalize_organization(
    organization: Organization,
    options: Option<Options>,
) -> Result<Normalization, Errors> {
    log::trace!("In normalize_organization");

    let Organization {
        basis_graph,
        organized_data,
        related_data
    } = organization;

    let normalized_data = basis_graph.normalize(organized_data).await;

    Normalization {
        basis_graph,
        related_data,
        normalized_data,
    }
}
