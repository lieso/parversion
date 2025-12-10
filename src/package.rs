use crate::document::Document;
use crate::mutation::Mutation;
use crate::document_format::DocumentFormat;

pub struct Package {
    pub document: Document,
    pub mutations: Vec<Mutation>,
}

impl Package {
    pub fn to_string(&self, document_format: &Option<DocumentFormat>) -> String {
        self.document.to_string(document_format)
    }
}
