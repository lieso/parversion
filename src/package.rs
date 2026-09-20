use crate::document::Document;

pub struct Package {
    pub document: Document,
}

impl Package {
    pub fn to_string(&self) -> String {
        self.document.to_string()
    }
}
