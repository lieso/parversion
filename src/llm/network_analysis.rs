use openrouter_rs::{
    api::chat::*,
    types::{ResponseFormat, Role},
    OpenRouterClient,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::prelude::*;
use crate::environment::get_env_variable;

pub struct NetworkAnalysis;

pub struct NetworkTransformationResponse {

}

impl NetworkAnalysis {
    pub async fn get_network_transformation(
        json: &str,
        document_summary: &str
    ) -> Result<NetworkTransformationResponse, Errors> {
        log::trace!("In get_netwwork_transformation");

        unimplemented!()
    }
}
