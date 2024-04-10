use crate::models;

pub enum Errors {
}

pub async fn get_parsers(html: &str) -> Result<Vec<models::curated_listing::CuratedListingParser2>, Errors> {
    log::trace!("In get_parsers");

    panic!("testing");
}
