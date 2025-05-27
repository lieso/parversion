use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InterfaceType {
    id: ID,
    name: String,
    description: String,
}
