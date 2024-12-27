use async_trait::async_trait;

use crate::prelude::*;
use crate::document_profile::DocumentProfile;

#[async_trait]
pub trait Provider: Send + Sync + Sized {
    async fn get_document_profile(&self, features: &str) -> Result<Option<DocumentProfile>, Errors>;

    //async fn get_basis_node(&self, lineage: Lineage) -> Result<Option<BasisNode>, Errors>;

    //async fn get_basis_graph(&self) -> Result<Option<BasisGraph>, Errors>;
}


