use std::sync::{Arc};

use crate::prelude::*;
use crate::transformation::FieldTransformation;
use crate::meta_context::MetaContext;
use crate::context::{Context, ContextID};

mod openai;


pub struct LLM {}

impl LLM {
    pub async fn get_field_transformation(
        meta_context: Arc<MetaContext>,
        context_group: Vec<Arc<Context>>,
    ) -> Result<Option<FieldTransformation>, Errors> {
        unimplemented!()
        //openai::OpenAI::get_field_transformation(field, value, snippet).await
    }
}
