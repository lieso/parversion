use async_trait::async_trait;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::sync::RwLock as AsyncRwLock;
use std::collections::{HashSet};
use serde_yaml;

use crate::prelude::*;
use crate::profile::Profile;
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;
use crate::basis_graph::BasisGraph;
use crate::schema::Schema;
use crate::transformation::SchemaTransformation;

pub mod yaml;

#[async_trait]
pub trait Provider: Send + Sync + Sized + 'static {
    async fn get_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors>;
    async fn save_profile(
        &self,
        profile: &Profile
    ) -> Result<(), Errors>;
    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors>;
    async fn save_basis_node(
        &self,
        lineage: &Lineage,
        basis_node: BasisNode,
    ) -> Result<(), Errors>;
    async fn get_basis_network_by_subgraph_hash(
        &self,
        subgraph_hash: &String
    ) -> Result<Option<BasisNetwork>, Errors>;
    async fn save_basis_network(
        &self,
        subgraph_hash: String,
        basis_network: BasisNetwork
    ) -> Result<(), Errors>;
    async fn get_basis_graph_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisGraph>, Errors>;
    async fn save_basis_graph(
        &self,
        lineage: &Lineage,
        basis_graph: BasisGraph
    ) -> Result<(), Errors>;
    async fn get_schema_transformation(
        &self,
        lineage: &Lineage,
        target: Option<&Hash>
    ) -> Result<Option<SchemaTransformation>, Errors>;
    async fn save_schema_transformation(
        &self,
        lineage: &Lineage,
        schema_transformation: SchemaTransformation
    ) -> Result<(), Errors>;
}

pub struct VoidProvider;

#[async_trait]
impl Provider for VoidProvider {
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
        _schema_transformation: SchemaTransformation
    ) -> Result<(), Errors> {
        Ok(())
    }
}
