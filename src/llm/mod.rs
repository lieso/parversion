use crate::transformation::FieldTransformation;

mod openai;

pub struct LLM {}

impl LLM {
    pub async fn get_field_transformation(
        field: &str,
        value: &str,
        snippet: &str,
    ) -> Option<FieldTransformation> {
        openai::OpenAI::get_field_transformation(field, value, snippet).await
    }
}
