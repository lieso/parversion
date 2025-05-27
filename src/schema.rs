use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schema {
    id: ID,
    name: String,
    description: String,
}
