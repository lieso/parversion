use std::sync::{Arc};
use serde::{Serialize, Deserialize};
use std::collections::{VecDeque};
use uuid::Uuid;

use crate::graph_node::{Graph, get_lineage, apply_lineage, GraphNodeData};
use crate::xml_node::{XmlNode};
use crate::basis_node::{BasisNode};
use crate::error::{Errors};
use crate::macros::*;
use crate::basis_graph::{BasisGraph};
use crate::node_data_structure::{apply_structure};


