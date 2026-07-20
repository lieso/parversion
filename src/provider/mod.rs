use async_trait::async_trait;

use crate::document::Document;
use crate::classification::Classification;
use crate::basis_network::BasisNetwork;
use crate::basis_group::BasisGroup;
use crate::basis_node::BasisNode;
use crate::basis_graph::BasisGraph;
use crate::basis_field::BasisField;
use crate::operation::Operation;
use crate::translation_node::TranslationNode;
use crate::translation_network::TranslationNetwork;
use crate::prelude::*;

#[cfg(feature = "yaml-provider")]
pub mod yaml;

#[cfg(feature = "sqlite-provider")]
pub mod sqlite;

#[async_trait]
pub trait Provider: Send + Sync + Sized + 'static {
    async fn get_basis_fields_by_acyclic_subgraph_hash(
        &self,
        acyclic_subgraph_hash: &Hash
    ) -> Result<Vec<BasisField>, Errors>;
    async fn save_basis_fields(
        &self,
        acyclic_subgraph_hash: &Hash,
        basis_fields: Vec<BasisField>
    ) -> Result<(), Errors>;
    async fn get_basis_groups_by_acyclic_lineage(
        &self,
        acyclic_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors>;
    async fn get_basis_groups_by_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors>;
    async fn get_basis_groups_by_indexed_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
        indexed_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors>;
    async fn save_basis_group(
        &self,
        acyclic_lineage: &Lineage,
        lineage: Option<&Lineage>,
        indexed_lineage: Option<&Lineage>,
        basis_group: BasisGroup,
    ) -> Result<(), Errors>;
    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<BasisNode>, Errors>;
    async fn save_basis_node(&self, lineage: &Lineage, basis_node: BasisNode)
        -> Result<(), Errors>;
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
    async fn get_basis_graph_by_hash(
        &self,
        hash: &Hash
    ) -> Result<Option<BasisGraph>, Errors>;
    async fn save_basis_graph(
        &self,
        hash: &Hash,
        basis_graph: BasisGraph,
    ) -> Result<(), Errors>;
    async fn save_schema_instance_document(
        &self,
        hash: &Hash,
        document: Document
    ) -> Result<(), Errors>;
    async fn get_instance_document_by_schema_hash(
        &self,
        hash: &Hash
    ) -> Result<Option<Document>, Errors>;
    async fn get_translation_node_by_lineages(
        &self,
        lineage_from: &Lineage,
        lineage_to: &Lineage
    ) -> Result<Option<Option<TranslationNode>>, Errors>;
    async fn save_translation_node(
        &self,
        lineages: (Lineage, Lineage),
        translation_node: Option<TranslationNode>
    ) -> Result<(), Errors>;
    async fn get_translation_network_by_lineages(
        &self,
        lineage_from: &Lineage,
        lineage_to: &Lineage
    ) -> Result<Option<Option<TranslationNetwork>>, Errors>;
    async fn save_translation_network(
        &self,
        lineages: (Lineage, Lineage),
        translation_network: Option<TranslationNetwork>
    ) -> Result<(), Errors>;
}

pub struct VoidProvider;

#[async_trait]
impl Provider for VoidProvider {
    async fn get_basis_groups_by_acyclic_lineage(
        &self,
        _acyclic_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        Ok(Vec::new())
    }
    async fn get_basis_groups_by_lineage(
        &self,
        _acyclic_lineage: &Lineage,
        _lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        Ok(Vec::new())
    }
    async fn get_basis_groups_by_indexed_lineage(
        &self,
        _acyclic_lineage: &Lineage,
        _lineage: &Lineage,
        _indexed_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        Ok(Vec::new())
    }
    async fn save_basis_group(
        &self,
        _acyclic_lineage: &Lineage,
        _lineage: Option<&Lineage>,
        _indexed_lineage: Option<&Lineage>,
        _basis_group: BasisGroup,
    ) -> Result<(), Errors> {
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

    async fn get_basis_fields_by_acyclic_subgraph_hash(
        &self,
        acyclic_subgraph_hash: &Hash
    ) -> Result<Vec<BasisField>, Errors> {
        Ok(Vec::new())
    }

    async fn save_basis_fields(
        &self,
        acyclic_subgraph_hash: &Hash,
        basis_fields: Vec<BasisField>
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn save_schema_instance_document(
        &self,
        _hash: &Hash,
        _document: Document
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_instance_document_by_schema_hash(
        &self,
        _hash: &Hash
    ) -> Result<Option<Document>, Errors> {
        Ok(None)
    }

    async fn get_translation_node_by_lineages(
        &self,
        _lineage_from: &Lineage,
        _lineage_to: &Lineage
    ) -> Result<Option<Option<TranslationNode>>, Errors> {
        Ok(None)
    }

    async fn save_translation_node(
        &self,
        _lineages: (Lineage, Lineage),
        _translation_node: Option<TranslationNode>
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_translation_network_by_lineages(
        &self,
        lineage_from: &Lineage,
        lineage_to: &Lineage
    ) -> Result<Option<Option<TranslationNetwork>>, Errors> {
        Ok(None)
    }

    async fn save_translation_network(
        &self,
        lineages: (Lineage, Lineage),
        translation_network: Option<TranslationNetwork>
    ) -> Result<(), Errors> {
        Ok(())
    }
}
