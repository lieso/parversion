use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};

use crate::prelude::*;
use crate::provider::Provider;
use crate::graph_node::{GraphNode};
use crate::context::{Context};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Interface {
    id: ID,
    name: String,
    description: String,
}

impl Interface {
    pub async fn get_interface<P: Provider>(
        provider: Arc<P>,
        contexts: &HashMap<ID, Arc<Context>>,
        graph_root: &Arc<RwLock<GraphNode>>,
    ) -> Result<Arc<Interface>, Errors> {
        log::trace!("In get_interface");

        let interfaces: Vec<Interface> = provider.list_interfaces().await?;

        if interfaces.len() == 0 {
            unimplemented!();
        }

        if interfaces.len() == 1 {
            return Ok(Arc::new(interfaces[0].clone()));
        }

        unimplemented!()
    }
}
