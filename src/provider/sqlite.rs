use async_trait::async_trait;
use std::collections::{HashSet};

use crate::prelude::*;
use crate::profile::Profile;
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;
use crate::basis_graph::BasisGraph;
use crate::transformation::SchemaTransformation;

#[cfg(feature = "sqlite-provider")]
pub struct SqliteProvider {

}

#[cfg(feature = "sqlite-provider")]
impl SqliteProvider {
    pub fn new() -> Self {
        Self {

        }
    }
}

#[cfg(feature = "yaml-provider")]
#[async_trait]
impl Provider for YamlFileProvider {
    async fn get_profile(
        &self,
        _features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors> {
        Ok(None)
    }

    async fn save_profile(
        &self,
        _profile: &Profile
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_basis_node_by_lineage(
        &self,
        _lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors> {
        Ok(None)
    }

    async fn save_basis_node(
        &self,
        _lineage: &Lineage,
        _basis_node: BasisNode,
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_basis_network_by_subgraph_hash(
        &self,
        _subgraph_hash: &String
    ) -> Result<Option<BasisNetwork>, Errors> {
        Ok(None)
    }

    async fn save_basis_network(
        &self,
        _subgraph_hash: String,
        _basis_network: BasisNetwork
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_basis_graph_by_lineage(
        &self,
        _lineage: &Lineage
    ) -> Result<Option<BasisGraph>, Errors> {
        Ok(None)
    }

    async fn save_basis_graph(
        &self,
        _lineage: &Lineage,
        _basis_graph: BasisGraph
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_schema_transformation(
        &self,
        _lineage: &Lineage,
        _target: Option<&Hash>
    ) -> Result<Option<SchemaTransformation>, Errors> {
        Ok(None)
    }

    async fn save_schema_transformation(
        &self,
        _lineage: &Lineage,
        _target_schema: Option<&Hash>,
        _schema_transformation: SchemaTransformation
    ) -> Result<(), Errors> {
        Ok(())
    }
}
