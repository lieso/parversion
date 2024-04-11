use sled::{Db};
use bincode::{serialize, deserialize};
use std::error::Error;

use crate::models::tree::*;

pub fn store_node_data(db: &Db, key: &str, nodes: Vec<NodeData>) -> Result<(), Box<dyn Error>> {
    let serialized_nodes = serialize(&nodes)?;
    db.insert(key, serialized_nodes)?;
    Ok(())
}

pub fn get_node_data(db: &Db, key: &str) -> Result<Option<Vec<NodeData>>, Box<dyn Error>> {
    match db.get(key)? {
        Some(serialized_nodes) => {
            let nodes_data: Vec<NodeData> = deserialize(&serialized_nodes)?;
            Ok(Some(nodes_data))
        },
        None => Ok(None),
    }
} 












