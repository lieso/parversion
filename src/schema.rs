use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::{HashMap};

use crate::prelude::*;
use crate::transformation::SchemaTransformation;
use crate::provider::Provider;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schema {
    id: ID,
    name: String,
    description: String,
}

impl Schema {
    pub fn from_meta_context(
        meta_context: Arc<RwLock<MetaContext>>,
    ) -> Result<Self, Errors> {
        log::trace!("In from_meta_context");

        unimplemented!()
    }

    pub async fn get_schema_transformations<P: Provider>(
        &self,
        provider: Arc<P>,
        target_schema: Arc<Schema>,
    ) -> Result<HashMap<ID, Arc<SchemaTransformation>>, Errors> {
        log::trace!("In get_schema_transformations");

        unimplemented!()
    }

    pub async fn new_normal_schema<P: Provider>(
        &self,
        provider: Arc<P>,
    ) -> Result<(Self, HashMap<ID, Arc<SchemaTransformation>>), Errors> {
        log::trace!("In new_normal_schema");

        unimplemented!()
    }
}
