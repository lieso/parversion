use crate::document::Document;
use crate::mutations::Mutations;
use crate::document_format::DocumentFormat;

pub struct Package {
    pub document: Document,
    pub mutations: Mutations,
}

impl Package {
    pub fn to_string(&self, document_format: &Option<DocumentFormat>) -> String {
        unimplemented!()
    }
}
