use crate::document::Document;
use crate::document_format::DocumentFormat;
use crate::mutation::Mutation;

pub struct Package {
    pub document: Document,
    pub mutations: Vec<Mutation>,
}

impl Package {
    pub fn to_string(&self) -> String {
        self.document.to_string()
    }
}
