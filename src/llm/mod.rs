use std::sync::{Arc, RwLock};
use std::collections::{HashSet, VecDeque};

use crate::prelude::*;
use crate::transformation::FieldTransformation;
use crate::meta_context::MetaContext;
use crate::context::{Context, ContextID};
use crate::graph_node::{GraphNode, GraphNodeID};
use crate::config::{CONFIG};

mod openai;

pub struct LLM {}

impl LLM {
    pub async fn get_field_transformations(
        meta_context: Arc<MetaContext>,
        context_group: Vec<Arc<Context>>,
    ) -> Result<Vec<FieldTransformation>, Errors> {
        log::trace!("In get_field_transformation");

        let example_snippet_count = read_lock!(CONFIG).llm.example_snippet_count;

        let snippets: Vec<String> = context_group
            .iter()
            .take(example_snippet_count)
            .map(|context| context.generate_snippet(Arc::clone(&meta_context)))
            .collect();

        unimplemented!()
        //openai::OpenAI::get_field_transformation(field, value, snippet).await
    }
}
