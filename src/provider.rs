use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::fs:
use serde_json::Value;
use rusqlite::{Connection};
use serde_yaml;

use crate::prelude::*;
use crate::document_profile::DocumentProfile;

#[async_trait]
pub trait Provider: Send + Sync + Sized {
    async fn get_document_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<&DocumentProfile>, Errors>;
}

pub struct VoidProvider;

impl Provider for VoidProvider {
    async fn get_document_profile(
        &self,
        _features: &HashSet<Hash>
    ) -> Result<Option<&DocumentProfile>, Errors> {
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
    async fn get_document_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<&DocumentProfile>, Errors> {
        let data = fs::read_to_string(&self.file_path)
            .map_err(|_| Errors::FileReadError)?;

        let yaml: serde_yaml::Value = serde_yaml::from_str(&data)
            .map_err(|_| Errors::YamlParseError)?;

        let document_profiles: Vec<DocumentProfile> = yaml.get("document_profiles")
            .and_then(|dp| serde_yaml::from_value(dp.clone()).ok())
            .ok_or(Errors::YamlParseError)?;

        if let Some(target_profile) = DocumentProfile::get_similar_profile(
            &document_profiles,
            features
        ) {
            Ok(Some(target_profile))
        } else {
            Ok(None)
        }
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
    async fn get_document_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<&DocumentProfile>, Errors> {
        let data = fs::read_to_string(&self.file_path)
            .map_err(|_| Errors::FileReadError)?;

        let json: Value = serde_json::from_str(&data)
            .map_err(|_| Errors::JsonParseError)?;

        let document_profiles: Vec<DocumentProfile> = json.get("document_profiles")
            .and_then(|dp| serde_json::from_value(dp.clone()).ok())
            .ok_or(Errors::JsonParseError)?;

        if let Some(target_profile) = DocumentProfile::get_similar_profile(
            &document_profiles,
            features
        ) {
            Ok(Some(target_profile))
        } else {
            Ok(None)
        }
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

#[async_trait]
impl Provider for SqliteProvider {
    async fn get_document_profiles(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<&DocumentProfile>, Errors> {
        let conn = Connection::open(&self.db_path)
            .map_err(|_| Errors::SqliteDatabaseConnectionError)?;

        unimplemented!()
    }
}
