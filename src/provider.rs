use async_trait::async_trait;

pub trait Provider {
    async fn get_document_profile(&self, features: &str) -> Result<Option<DocumentProfile>>, Errors>;

    async fn get_basis_node(&self, lineage: Lineage) -> Result<Option<BasisNode>, Errors>;

    async fn get_basis_graph(&self) -> Result<Option<BasisGraph>, Errors>;
}
