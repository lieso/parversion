use async_trait::async_trait;
use std::collections::HashSet;

use crate::classification::Classification;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::operation::Operation;
use crate::prelude::*;
use crate::profile::Profile;
use crate::transformation::SchemaTransformation;

#[cfg(feature = "yaml-provider")]
pub mod yaml;

#[cfg(feature = "sqlite-provider")]
pub mod sqlite;

#[async_trait]
pub trait Provider: Send + Sync + Sized + 'static {
    async fn get_profile(&self, features: &HashSet<Hash>) -> Result<Option<Profile>, Errors>;
    async fn save_profile(&self, profile: &Profile) -> Result<(), Errors>;
    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<BasisNode>, Errors>;
    async fn save_basis_node(&self, lineage: &Lineage, basis_node: BasisNode)
        -> Result<(), Errors>;
    async fn get_basis_network_by_lineage_and_subgraph_hash(
        &self,
        lineage: &Lineage,
        subgraph_hash: &Hash,
    ) -> Result<Option<BasisNetwork>, Errors>;
    async fn save_basis_network(
        &self,
        lineage: &Lineage,
        subgraph_hash: &Hash,
        basis_network: BasisNetwork,
    ) -> Result<(), Errors>;
    async fn get_classification_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<Classification>, Errors>;
    async fn save_classification(
        &self,
        lineage: &Lineage,
        classification: Classification,
    ) -> Result<(), Errors>;
    async fn get_schema_transformation(
        &self,
        lineage: &Lineage,
        target_schema: Option<&Hash>,
    ) -> Result<Option<SchemaTransformation>, Errors>;
    async fn save_schema_transformation(
        &self,
        lineage: &Lineage,
        target_schema: Option<&Hash>,
        schema_transformation: SchemaTransformation,
    ) -> Result<(), Errors>;
    async fn get_operation_by_hash(&self, hash: &Hash) -> Result<Option<Operation>, Errors>;
    async fn save_operation(&self, hash: &Hash, operation: Operation) -> Result<(), Errors>;
}

pub struct VoidProvider;

#[async_trait]
impl Provider for VoidProvider {
    async fn get_profile(&self, _features: &HashSet<Hash>) -> Result<Option<Profile>, Errors> {
        Ok(None)
    }

    async fn save_profile(&self, _profile: &Profile) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_basis_node_by_lineage(
        &self,
        _lineage: &Lineage,
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

    async fn get_basis_network_by_lineage_and_subgraph_hash(
        &self,
        _lineage: &Lineage,
        _subgraph_hash: &Hash,
    ) -> Result<Option<BasisNetwork>, Errors> {
        Ok(None)
    }

    async fn save_basis_network(
        &self,
        _lineage: &Lineage,
        _subgraph_hash: &Hash,
        _basis_network: BasisNetwork,
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_classification_by_lineage(
        &self,
        _lineage: &Lineage,
    ) -> Result<Option<Classification>, Errors> {
        Ok(None)
    }

    async fn save_classification(
        &self,
        _lineage: &Lineage,
        _classification: Classification,
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_schema_transformation(
        &self,
        _lineage: &Lineage,
        _target: Option<&Hash>,
    ) -> Result<Option<SchemaTransformation>, Errors> {
        Ok(None)
    }

    async fn save_schema_transformation(
        &self,
        _lineage: &Lineage,
        _target_schema: Option<&Hash>,
        _schema_transformation: SchemaTransformation,
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_operation_by_hash(&self, hash: &Hash) -> Result<Option<Operation>, Errors> {
        Ok(None)
    }

    async fn save_operation(&self, hash: &Hash, operation: Operation) -> Result<(), Errors> {
        Ok(())
    }
}
