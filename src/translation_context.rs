use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::normal_context::NormalContext;
use crate::normal_meta_context::NormalMetaContext;
use crate::prelude::*;
use crate::translation_node::TranslationNode;

pub struct TranslationContext {
    pub source: Vec<Arc<RwLock<NormalizationContext>>>,
    pub target: Vec<Arc<RwLock<NormalizationContext>>>,
    pub translation_nodes: Option<HashMap<TranslationNodeID, Arc<TranslationNode>>>,
    pub translation_nodes_lookup: Option<HashMap<Lineage, Arc<TranslationNode>>>,
    pub target_node_lookup: Option<HashMap<Lineage, Vec<Arc<NormalContext>>>>,
    pub translated: Option<NormalMetaContext>,
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
            target_node_lookup: None,
            translated: None,
        }
    }

    pub fn update_translation_nodes(
        &mut self,
        translation_nodes: HashMap<TranslationNodeID, Arc<TranslationNode>>,
        lookup: HashMap<Lineage, Arc<TranslationNode>>,
        target_lookup: HashMap<Lineage, Vec<Arc<NormalContext>>>,
    ) {
        self.translation_nodes = Some(translation_nodes);
        self.translation_nodes_lookup = Some(lookup);
        self.target_node_lookup = Some(target_lookup);
    }

    pub fn update_translated_graph(&mut self, translated: NormalMetaContext) {
        self.translated = Some(translated);
    }
}
