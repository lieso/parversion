use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::fs;
use serde_json::Value;
use serde_yaml;

use crate::prelude::*;
use crate::profile::Profile;
use crate::basis_node::BasisNode;

#[async_trait]
pub trait Provider: Send + Sync + Sized + 'static {
    async fn get_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors>;
    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors>;
}

pub struct VoidProvider;

#[async_trait]
impl Provider for VoidProvider {
    async fn get_profile(
        &self,
        _features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors> {
        Ok(None)
    }

    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors> {
        Ok(None)
    }
}

pub struct YamlFileProvider {
    file_path: String,
}

impl YamlFileProvider {
    pub fn new(file_path: String) -> Self {
        YamlFileProvider { file_path }
    }
}

#[async_trait]
impl Provider for YamlFileProvider {
    async fn get_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors> {
        let data = fs::read_to_string(&self.file_path)
            .map_err(|_| Errors::FileReadError)?;

        let serialized_features = serde_yaml::to_string(features).expect("Could not serialize to yaml");

        log::debug!("serialized_features: {}", serialized_features);

        let yaml_result: Result<serde_yaml::Value, _> = serde_yaml::from_str(&data);
        if let Err(e) = yaml_result {
            log::error!("Failed to parse YAML: {:?}", e);
            return Err(Errors::YamlParseError);
        }
        let yaml = yaml_result.unwrap();

        let profiles: Vec<Profile> = yaml.get("profiles")
            .and_then(|dp| {
                let deserialized: Result<Vec<Profile>, _> = serde_yaml::from_value(dp.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error: {:?}", err);
                }
                deserialized.ok()
            })
            .ok_or(Errors::YamlParseError)?;

        if let Some(target_profile) = Profile::get_similar_profile(
            &profiles,
            features
        ) {
            Ok(Some(target_profile))
        } else {
            Ok(None)
        }
    }

    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors> {
        Ok(None)
    }
}

pub struct JsonFileProvider {
    file_path: String,
}

impl JsonFileProvider {
    pub fn new(file_path: String) -> Self {
        JsonFileProvider { file_path }
    }
}

#[async_trait]
impl Provider for JsonFileProvider {
    async fn get_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors> {
        let data = fs::read_to_string(&self.file_path)
            .map_err(|_| Errors::FileReadError)?;

        let json: Value = serde_json::from_str(&data)
            .map_err(|_| Errors::JsonParseError)?;

        let profiles: Vec<Profile> = json.get("profiles")
            .and_then(|dp| serde_json::from_value(dp.clone()).ok())
            .ok_or(Errors::JsonParseError)?;

        if let Some(target_profile) = Profile::get_similar_profile(
            &profiles,
            features
        ) {
            Ok(Some(target_profile))
        } else {
            Ok(None)
        }
    }

    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors> {
        Ok(None)
    }
}

pub struct SqliteProvider {
    db_path: String,
}

impl SqliteProvider {
    pub fn new(db_path: String) -> Self {
        SqliteProvider { db_path }
    }
}
