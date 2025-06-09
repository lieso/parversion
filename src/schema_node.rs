use serde::{Serialize, Deserialize};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaNode {
    #[serde(skip_serializing)]
    pub id: ID,
    #[serde(skip_serializing)]
    pub name: String,
    pub description: String,
    pub data_type: String,
    pub properties: HashMap<String, SchemaNode>,
}





// include basis graph name as root key on schema



//digest.baz.qux.user_name -> digest.baz.qux.user_name

//digest.foo.bar.username -> digest.baz.qux.user_name

//digest.one.two.user_name -> digest.foo.bar.username

//digest.one.two.user_name -> digest.three.four.username
