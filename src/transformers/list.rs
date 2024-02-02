use crate::models;

pub fn transform_document_to_list(document: String, parser: &models::list::ListParser) -> models::list::List {
    log::trace!("In transform_document_to_list");

    let list = models::list::List {
        items: Vec::new(),
    };

    return list;
}
