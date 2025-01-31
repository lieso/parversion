use crate::transformation::FieldTransformation;

mod openai;

pub struct LLM {}

impl LLM {
    pub async fn get_field_transformation(
        field: String,
        value: String,
        snippet: String,
    ) -> FieldTransformation {
        openai::OpenAI::get_field_transformation(field, value, snippet).await
    }
}
