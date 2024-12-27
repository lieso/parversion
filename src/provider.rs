use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

use crate::prelude::*;
use crate::document_profile::DocumentProfile;

#[async_trait]
pub trait Provider: Send + Sync + Sized {
    async fn get_document_profile(&self, features: &HashSet<u64>) -> Result<Option<&DocumentProfile>, Errors>;
}


