use crate::document::Document;
use crate::mutations::Mutations;

pub struct Package {
    pub document: Document,
    pub mutations: Mutations,
}

impl Package {
    pub fn to_string(&self) -> String {
        unimplemented!()
    }
}
