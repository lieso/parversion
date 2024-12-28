use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};

use crate::id::{ID};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Runtime {
    AWK,
    NodeJS,
    Python,
    QuickJS,
}

trait Transform {
    fn get_id(&self) -> ID;
    fn get_runtime(&self) -> Runtime;
    fn get_code(&self) -> String;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JsonSchemaTransform {
    id: ID,
    runtime: Runtime,
    code: String,
    source: String,
    target: String,
}

impl Transform for JsonSchemaTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNodeFieldsTransform {
    id: ID,
    runtime: Runtime,
    code: String,
}

impl Transform for DataNodeFieldsTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNodeHashTransform {
    id: ID,
    runtime: Runtime,
    regex: String,
    code: String,
}

impl Transform for DataNodeHashTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataNodeRecursiveTransform {
    id: ID,
    runtime: Runtime,
    code: String,
}

impl Transform for DataNodeRecursiveTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataToJsonFieldTransform {
    id: ID,
    runtime: Runtime,
    code: String,
}

impl Transform for DataToJsonFieldTransform {
    fn get_id(&self) -> ID {
        self.id.clone()
    }

    fn get_runtime(&self) -> Runtime {
        self.runtime.clone()
    }

    fn get_code(&self) -> String {
        self.code.clone()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Transformation {
    DataNodeFieldsTransform(DataNodeFieldsTransform),
    DataNodeRecursiveTransform(DataNodeRecursiveTransform),
    DataNodeHashTransform(DataNodeHashTransform),
    DataToJsonFieldTransform(DataToJsonFieldTransform),
    JsonSchemaTransform(JsonSchemaTransform),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct XMLElementTransformation {
    pub id: ID,
    pub description: String,
    pub runtime: Runtime,
    pub code: String,
}

impl XMLElementTransformation {
    pub fn get_signature(element: String, attributes: HashMap<String, String>) -> String {
        let element_code = format!("let element = '{}';", element);

        let attributes_code = {
            let attributes_list: Vec<String> = attributes
                .into_iter()
                .map(|(key, value)| format!("'{}': '{}'", key, value))
                .collect();
            format!("let attributes = {{ {} }};", attributes_list.join(", "))
        };

        format!("{}\n{}", element_code, attributes_code)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DocumentTransformation {
    XMLElementTransformation(XMLElementTransformation),
}

//pub fn transform<T, U>(transformation: Transformation, payload: T) -> U {
    //match transformation {
    //    Transformation::DataNodeFieldsTransform(t) => {
    //        match t.get_runtime() {
    //            Runtime::Python => runtimes::python_field_map(&t.get_code(), payload)
    //        }
    //    },
    //    Transformation::DataNodeHashTransform(t) => {
    //        match t.get_runtime() {
    //            Runtime::Python => runtimes::python_field_constant(&t.get_code(), payload)
    //        }
    //    },
    //    _ => unimplemented!()
    //}
//}

//lazy_static! {
//    pub static ref VALUE_TRANSFORMATIONS: Vec<Transformation> = vec![
//        Transformation {
//            runtime: Runtime::AWK,
//            description: String::from("Converts American weights in pounds (lbs)"),
//            regex: r"\b\d+(\.\d+)?\s*(lbs?|pounds?)\b",
//            code: String::from(r#"{ printf "%.2f lbs = %.2f kg\n", $1, $1 * 0.45359237 }"#),
//        },
//        Transformation {
//            runtime: Runtime::AWK,
//            description: String::from("Identity Transformation"),
//            regex: r"(?s).*",
//            code: String::from(r#"{ print $0 }"#),
//        },
//    ];
//}
