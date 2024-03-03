use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DocumentType {
    Article,
    LongForm,
    Chat,
    Weather,
    BusinessDetails,
    CuratedListing,
    EventListing,
    JobListing,
    RealEstateListing,
    SearchEngineListing,
}
