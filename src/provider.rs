use async_trait::async_trait;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::sync::RwLock as AsyncRwLock;
use std::collections::{HashSet};
use serde_yaml;

use crate::prelude::*;
use crate::profile::Profile;
use crate::basis_node::BasisNode;
use crate::basis_network::BasisNetwork;
use crate::interface_type::InterfaceType;
use crate::schema::Schema;

#[async_trait]
pub trait Provider: Send + Sync + Sized + 'static {
    async fn list_interfaces(
        &self,
    ) -> Result<Vec<Interface>, Errors>;
    async fn save_interface(
        &self,
        interface: &Interface,
    ) -> Result<(), Errors>;
    async fn get_schema_by_interface(
        &self,
        interface: &Interface
    ) -> Result<Option<Schema>, Errors>;
    async fn save_schema_for_interface(
        &self,
        interface: &Interface,
        schema: &Schema
    ) -> Result<(), Errors>;
    async fn get_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors>;
    async fn save_profile(
        &self,
        profile: &Profile
    ) -> Result<(), Errors>;
    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors>;
    async fn save_basis_node(
        &self,
        lineage: &Lineage,
        basis_node: BasisNode,
    ) -> Result<(), Errors>;
    async fn get_basis_network_by_subgraph_hash(
        &self,
        subgraph_hash: &String
    ) -> Result<Option<BasisNetwork>, Errors>;
    async fn save_basis_network(
        &self,
        subgraph_hash: String,
        basis_network: BasisNetwork
    ) -> Result<(), Errors>;
}

pub struct YamlFileProvider {
    file_path: String,
    cache: Arc<AsyncRwLock<Option<serde_yaml::Value>>>,
}

impl YamlFileProvider {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            cache: Arc::new(AsyncRwLock::new(None)),
        }
    }

    async fn load_data(&self) -> Result<serde_yaml::Value, Errors> {
        let mut cache = self.cache.write().await;
        if cache.is_none() {
            let data = async_fs::read_to_string(&self.file_path).await.map_err(|_| Errors::FileReadError)?;
            let yaml: serde_yaml::Value = serde_yaml::from_str(&data).map_err(|_| Errors::YamlParseError)?;
            *cache = Some(yaml.clone());
            Ok(yaml)
        } else {
            Ok(cache.clone().unwrap())
        }
    }

    async fn save_data(&self, yaml: &serde_yaml::Value) -> Result<(), Errors> {
        let new_yaml_str = serde_yaml::to_string(yaml).map_err(|_| Errors::UnexpectedError)?;
        async_fs::write(&self.file_path, new_yaml_str).await.map_err(|_| Errors::UnexpectedError)?;
        let mut cache = self.cache.write().await;
        *cache = Some(yaml.clone());
        Ok(())
    }
}

#[async_trait]
impl Provider for YamlFileProvider {
    async fn get_profile(
        &self,
        features: &HashSet<Hash>
    ) -> Result<Option<Profile>, Errors> {
        let yaml = self.load_data().await?;

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

    async fn save_profile(
        &self,
        profile: &Profile
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let profiles = yaml.get_mut("profiles")
            .and_then(|profiles_value| profiles_value.as_sequence_mut())
            .ok_or(Errors::YamlParseError)?;

        let new_profile_yaml = serde_yaml::to_value(&profile).map_err(|_| Errors::UnexpectedError)?;
        profiles.push(new_profile_yaml);

        self.save_data(&yaml).await
    }

    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors> {
        let yaml = self.load_data().await?;

        let basis_nodes: Vec<BasisNode> = yaml.get("basis_nodes")
            .and_then(|bn| {
                let deserialized: Result<Vec<BasisNode>, _> = serde_yaml::from_value(bn.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        for basis_node in basis_nodes {
            if &basis_node.lineage == lineage {
                return Ok(Some(basis_node));
            }
        }

        Ok(None)
    }

    async fn save_basis_node(
        &self,
        _lineage: &Lineage,
        basis_node: BasisNode,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_node = serde_yaml::to_value(&basis_node)
            .map_err(|_| Errors::UnexpectedError)?;

        if let Some(basis_nodes) = yaml.get_mut("basis_nodes") {
            basis_nodes.as_sequence_mut()
                .ok_or(Errors::YamlParseError)?
                .push(serialized_basis_node);
        } else {
            yaml["basis_nodes"] = serde_yaml::Value::Sequence(
                vec![serialized_basis_node]
            );
        }

        self.save_data(&yaml).await
    }

    async fn get_basis_network_by_subgraph_hash(
        &self,
        subgraph_hash: &String
    ) -> Result<Option<BasisNetwork>, Errors> {
        let yaml = self.load_data().await?;

        let basis_networks: Vec<BasisNetwork> = yaml.get("basis_networks")
            .and_then(|bn| {
                let deserialized: Result<Vec<BasisNetwork>, _> = serde_yaml::from_value(bn.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        for basis_network in basis_networks {
            if basis_network.subgraph_hash == *subgraph_hash {
                return Ok(Some(basis_network));
            }
        }

        Ok(None)
    }

    async fn save_basis_network(
        &self,
        _subgraph_hash: String,
        basis_network: BasisNetwork
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_network = serde_yaml::to_value(&basis_network)
            .map_err(|_| Errors::UnexpectedError)?;

        if let Some(basis_networks) = yaml.get_mut("basis_networks") {
            basis_networks.as_sequence_mut()
                .ok_or(Errors::YamlParseError)?
                .push(serialized_basis_network);
        } else {
            yaml["basis_networks"] = serde_yaml::Value::Sequence(
                vec![serialized_basis_network]
            );
        }

        self.save_data(&yaml).await
    }
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

    async fn save_profile(
        &self,
        _profile: &Profile
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_basis_node_by_lineage(
        &self,
        _lineage: &Lineage
    ) -> Result<Option<BasisNode>, Errors> {
        Ok(None)
    }

    async fn save_basis_node(
        &self,
        _lineage: &Lineage,
        _basis_node: BasisNode,
    ) -> Result<(), Errors> {
        Ok(())
    }

    async fn get_basis_network_by_subgraph_hash(
        &self,
        _subgraph_hash: &String
    ) -> Result<Option<BasisNetwork>, Errors> {
        Ok(None)
    }

    async fn save_basis_network(
        &self,
        _subgraph_hash: String,
        _basis_network: BasisNetwork
    ) -> Result<(), Errors> {
        Ok(())
    }
}

