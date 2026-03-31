use async_trait::async_trait;
use rusqlite::{Connection, Result};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::basis_graph::BasisGraph;
use crate::classification::Classification;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::operation::Operation;
use crate::prelude::*;
use crate::profile::Profile;
use crate::provider::Provider;
use crate::transformation::SchemaTransformation;

#[cfg(feature = "sqlite-provider")]
pub struct SqliteProvider {
    file_path: String,
    connection: Arc<Mutex<Connection>>,
}

#[cfg(feature = "sqlite-provider")]
impl SqliteProvider {
    pub fn new(file_path: String) -> Self {
        let connection = Connection::open(&file_path).expect("Could not create sqlite connection");

        Self {
            file_path,
            connection: Arc::new(Mutex::new(connection)),
        }
    }
}

#[cfg(feature = "sqlite-provider")]
#[async_trait]
impl Provider for SqliteProvider {
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

    async fn get_operation_by_hash(&self, _hash: &Hash) -> Result<Option<Operation>, Errors> {
        Ok(None)
    }

    async fn save_operation(&self, _hash: &Hash, _operation: Operation) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_basis_graph_by_hash(&self, _hash: &Hash) -> Result<Option<BasisGraph>, Errors> {
        Ok(None)
    }

    async fn save_basis_graph(
        &self,
        _hash: &Hash,
        _basis_graph: BasisGraph,
    ) -> Result<(), Errors> {
        Ok(())
    }
}
