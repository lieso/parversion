use crate::transformations::{SchemaTransformation};
use crate::id::{ID};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasisGraph {
    pub id: ID,
    pub name: String,
    pub description: String,
    pub json_schema: String,
    pub nodes: HashMap<ID, BasisNode>,
    pub networks: HashMap<ID, BasisNetwork>,
}

pub async fn classify_text(
    text: String
) -> Option<BasisGraph> {
    unimplemented!()
}

pub async fn create_basis_graph(
    text: String
) -> BasisGraph {
    unimplemented!()
}

pub async fn classify_or_create_basis_graph(
    text: String
) -> BasisGraph {
    unimplemented!()
}

