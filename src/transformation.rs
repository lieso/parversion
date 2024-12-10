use crate::schema_path::{SchemaPath};

pub enum Runtime {
    AWK,
    JavaScript,
    Python
}

trait Transform {
    fn get_id(&self) -> ID,
    fn get_runtime(&self) -> Runtime,
    fn get_script(&self) -> String,
}

pub struct JsonSchemaTransform {
    id: ID,
    source: SchemaPath,
    target: SchemaPath,
}

pub struct DataNodeFieldsTransform {
    id: ID,
    runtime: Runtime,
    expression: String,
}

pub struct DataNodeRecursiveTransform {
    id: ID,
    runtime: Runtime,
    source: 
    expression: String,
}

pub struct DataToJsonFieldTransform {
    id: ID,
    runtime: Runtime,
    regex: Regex,
    expression: String,
}

pub struct DataNodeHashTransform {
    id: ID,
    runtime: Runtime,
    regex: Regex,
    expression: String,
}

pub enum Transformation {
    A(DataNodeMeaningfulTransform),
    B(DataNodeRecursiveTransform),
    C(DataNodeHashTransform),
    D(DataToJsonFieldTransform),
    E(JsonSchemaTransform),
}

pub struct NodeTransformation {
    id: ID,
    name: String,
    description: String,
    source_expression: String,
    target_expression: String,
}

pub struct DocumentTransformation {
    id: ID,
    document_type: DocumentType,
    runtime: Runtime,
    expression: String,
}

pub fn transform<T, U>(transformation: Transformation, payload: T) -> U {
    let runtime = transformation.get_runtime();

    match transformation {
        Transformation::DataNodeFieldsTransform(t) => {
            match runtime {
                Runtime::Python => runtimes::python_field_map(t.get_script(), payload)
            }
        },
        Transformation::DataNodeHashTransform(t) => {
            match runtime {
                Runtime::Python => runtimes::python_field_constant(t.get_script(), payload)
            }
        }
    }
}

lazy_static! {
    pub static ref VALUE_TRANSFORMATIONS: Vec<Transformation> = vec![
        Transformation {
            runtime: Runtime::AWK,
            description: String::from("Converts American weights in pounds (lbs)"),
            regex: Regex::new(r"\b\d+(\.\d+)?\s*(lbs?|pounds?)\b").unwrap(),
            expression: String::from(r#"{ printf "%.2f lbs = %.2f kg\n", $1, $1 * 0.45359237 }"#),
        },
        Transformation {
            runtime: Runtime::AWK,
            description: String::from("Identity Transformation"),
            regex: Regex::new(r"(?s).*").unwrap(),
            expression: String::from(r#"{ print $0 }"#),
        },
    ];
}
