#![forbid(unsafe_code)]

pub mod analysis;
pub mod basis_network;
pub mod basis_node;
pub mod basis_graph;
pub mod config;
pub mod context;
pub mod data_node;
pub mod document;
pub mod document_format;
pub mod document_profile;
pub mod environment;
pub mod hash;
pub mod id;
pub mod lineage;
pub mod macros;
pub mod model;
pub mod normalization;
pub mod organization;
pub mod provider;
pub mod runtimes;
pub mod transformation;
pub mod translation;
pub mod types;
pub mod prelude;
pub mod utility;
pub mod json_node;

use std::sync::Arc;

use crate::provider::{Provider, DefaultProvider};

use crate::Parversion<P: Provider> {
    provider: Arc<P>,
}

pub struct Parversion<P: Provider> {
    provider: Arc<P>,
}

impl Parversion<DefaultProvider> {
    pub fn new() -> Self {
        Parversion {
            provider: Arc::new(DefaultProvider),
        }
    }
}

impl<P: Provider> Parversion<P> {

    pub fn from_provider(provider: P) -> Self {
        Parversion {
            provider: Arc::new(provider),
        }
    }

    

}

