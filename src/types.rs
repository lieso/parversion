use std::sync::{Arc};

use crate::basis_graph::{BasisGraph};
use crate::transformation::{Transformation};
use crate::data_node::DataNode;

pub struct NodeSet {
    pub data_nodes: Vec<Arc<DataNode>>,
}

#[derive(Clone, Debug)]
pub enum AnalysisMode {
    DISABLED,
    SIMPLE,
    COMPLEX,
}

#[derive(Clone, Debug)]
pub enum Errors {
    FileReadError,
    FileInputError,
    FileOutputError,
    JsonParseError,
    DocumentNotProvided,
    UnexpectedDocumentType,
    UnexpectedError,
    UnexpectedOutputFormat,
    XmlParseError,
    BasisGraphBuildError(String),
    PathConversionError,
    SqliteDatabaseConnectionError,
    YamlParseError,
    FetchUrlError(String),
}

#[derive(Clone, Debug)]
pub struct Options {
    pub analysis_mode: Option<AnalysisMode>,
    pub origin: Option<String>,
    pub date: Option<String>,
    pub value_transformations: Option<Vec<Transformation>>
}

impl Default for Options {
    fn default() -> Self {
        Options {
            analysis_mode: Some(AnalysisMode::COMPLEX),
            origin: None,
            date: None,
            value_transformations: None,
        }
    }
}
