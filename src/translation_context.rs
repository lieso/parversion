use std::sync::{Arc, RwLock};

use crate::prelude::*;

pub struct TranslationContext {
    pub source: Vec<Arc<RwLock<NormalizationContext>>>,
    pub target: Vec<Arc<RwLock<NormalizationContext>>>,
}

impl TranslationContext {
    pub fn new(
        source: Vec<Arc<RwLock<NormalizationContext>>>,
        target: Vec<Arc<RwLock<NormalizationContext>>>,
    ) -> Self {
        Self {
            source,
            target,
        }
    }
}
