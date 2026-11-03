use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::translation_node::TranslationNode;

pub struct TranslationContext {
    pub source: Vec<Arc<RwLock<NormalizationContext>>>,
    pub target: Vec<Arc<RwLock<NormalizationContext>>>,
    pub translation_nodes: Option<HashMap<TranslationNodeID, Arc<TranslationNode>>>,
    pub translation_nodes_lookup: Option<HashMap<Lineage, Arc<TranslationNode>>>,
}

impl TranslationContext {
    pub fn new(
        source: Vec<Arc<RwLock<NormalizationContext>>>,
        target: Vec<Arc<RwLock<NormalizationContext>>>,
    ) -> Self {
        Self {
            source,
            target,
            translation_nodes: None,
            translation_nodes_lookup: None,
        }
    }

    pub fn update_translation_nodes(
        &mut self,
        translation_nodes: HashMap<TranslationNodeID, Arc<TranslationNode>>,
        lookup: HashMap<Lineage, Arc<TranslationNode>>,
    ) {
        self.translation_nodes = Some(translation_nodes);
        self.translation_nodes_lookup = Some(lookup);
    }
}
