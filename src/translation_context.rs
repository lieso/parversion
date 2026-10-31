use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::prelude::*;
use crate::translation_node::TranslationNode;

pub struct TranslationContext {
    pub source: Vec<Arc<RwLock<NormalizationContext>>>,
    pub target: Vec<Arc<RwLock<NormalizationContext>>>,
    pub translation_nodes: Option<HashMap<TranslationNodeID, Arc<TranslationNode>>>,
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
        }
    }

    pub fn update_translation_nodes(
        &mut self,
        translation_nodes: HashMap<TranslationNodeID, Arc<TranslationNode>>,
    ) {
        self.translation_nodes = Some(translation_nodes);
    }
}
