use openrouter_rs::{
    api::chat::*,
    types::{ResponseFormat, Role},
    OpenRouterClient,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::prelude::*;
use crate::environment::get_env_variable;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeTransformationMetadata {
    pub tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeTransformationResponse {
    pub metadata: FieldTransformationResponseMetadata,
}

pub struct NodeAnalysis;

impl NodeAnalysis {

    pub async get_node_transformation(
        field: &str,
        value: &str,
    ) -> Result<NodeTransformationResponse, Errors> {
        log::trace!("In get_node_transformation");
    }

}
