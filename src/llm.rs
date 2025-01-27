use crate::prelude::*;
use crate::transformation::FieldTransformation;

pub struct LLM {}

impl LLM {
    pub async fn get_field_transformations(
        fields: Vec<String>,
        snippet: String,
    ) -> Vec<FieldTransformation> {
        log::trace!("In get_field_transformations");

        log::debug!("=====================================================================================================");
        log::debug!("=====================================================================================================");
        log::debug!("=====================================================================================================");

        log::debug!("fields: {:?}", fields);
        log::debug!("snippet: {}", snippet);


        unimplemented!()
    }
}
