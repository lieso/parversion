use std::sync::{Arc, RwLock};

use crate::prelude::*;

pub struct TranslationContext {
    pub normalization_contexts: Vec<Arc<RwLock<NormalizationContext>>>,
}

impl TranslationContext {
    pub fn new(normalization_contexts: Vec<Arc<RwLock<NormalizationContext>>>) -> Self {
        Self {
            normalization_contexts,
        }
    }
}
