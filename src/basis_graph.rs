use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use crate::prelude::*;
use crate::basis_node::{BasisNode};
use crate::basis_network::{BasisNetwork};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasisGraph {
    pub id: ID,
    pub name: String,
    pub description: String,
    pub json_schema: String,
    pub nodes: Vec<BasisNode>,
    pub networks: Vec<BasisNetwork>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasisGraphBuilder {
    id: ID,
    name: Option<String>,
    description: Option<String>,
    json_schema: Option<String>,
    nodes: HashMap<ID, BasisNode>,
    networks: Vec<BasisNetwork>,
}

impl BasisGraphBuilder {
    pub fn new() -> Self {
        BasisGraphBuilder {
            id: ID::new(),
            name: None,
            description: None,
            json_schema: None,
            nodes: HashMap::new(),
            networks: Vec::new(),
        }
    }

    pub fn from_basis_graph(basis_graph: &BasisGraph) -> Self {
        BasisGraphBuilder {
            id: basis_graph.id.clone(),
            name: Some(basis_graph.name.clone()),
            description: Some(basis_graph.description.clone()),
            json_schema: Some(basis_graph.json_schema.clone()),
            nodes: basis_graph.nodes.into_iter()
                .map(|node| (node.lineage.id.clone(), node.clone()))
                .collect(),
            networks: basis_graph.networks.clone(),
        }
    }

    pub fn build(self) -> Result<BasisGraph, Errors> {
        let name = self.name.ok_or_else(||
            Errors::BasisGraphBuilderError("Name is required".into())
        )?;
        let description = self.name.ok_or_else(||
            Errors::BasisGraphBuilderError("Description is required".into())
        )?;
        let json_schema = self.json_schema.ok_or_else(||
            Errors::BasisGraphBuilderError("JSON schema is required".into())
        )?;

        Ok(BasisGraph {
            id: self.id,
            name,
            description,
            json_schema,
            nodes: self.nodes,
            networks: self.networks,
        })
    }
}
