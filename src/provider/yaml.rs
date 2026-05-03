use async_trait::async_trait;
use serde_yaml;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::sync::RwLock as AsyncRwLock;

use crate::basis_graph::BasisGraph;
use crate::classification::Classification;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::bloom_filter::BloomFilter;
use crate::operation::Operation;
use crate::prelude::*;
use crate::profile::Profile;
use crate::provider::Provider;

#[cfg(feature = "yaml-provider")]
pub struct YamlFileProvider {
    file_path: String,
    cache: Arc<AsyncRwLock<Option<serde_yaml::Value>>>,
}

#[cfg(feature = "yaml-provider")]
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
            log::info!("Loading data from file: {}", &self.file_path);

            match async_fs::read_to_string(&self.file_path).await {
                Ok(data) => {
                    log::info!("Read yaml provider file successfully");

                    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&data).map_err(|e| {
                        log::error!("Failed to parse yaml: {}", e);
                        Errors::YamlProviderError
                    })?;

                    if !yaml.is_mapping() {
                        yaml = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                    }

                    *cache = Some(yaml.clone());
                    log::info!("YAML provider data loaded and cached successfully");

                    Ok(yaml)
                }
                Err(_) => {
                    log::info!(
                        "Failed to read yaml provider file. Will attempt to create one now..."
                    );

                    async_fs::File::create(&self.file_path).await.map_err(|_| {
                        log::error!("Failed to create yaml provider file: {}", &self.file_path);
                        Errors::YamlProviderError
                    })?;

                    log::info!("Initialized yaml provider with empty yaml");

                    let yaml = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                    *cache = Some(yaml.clone());

                    Ok(yaml)
                }
            }
        } else {
            Ok(cache.clone().unwrap())
        }
    }

    async fn save_data(&self, yaml: &serde_yaml::Value) -> Result<(), Errors> {
        let new_yaml_str = serde_yaml::to_string(yaml).map_err(|_| Errors::UnexpectedError)?;
        async_fs::write(&self.file_path, new_yaml_str)
            .await
            .map_err(|_| Errors::UnexpectedError)?;
        let mut cache = self.cache.write().await;
        *cache = Some(yaml.clone());
        Ok(())
    }
}

#[cfg(feature = "yaml-provider")]
#[async_trait]
impl Provider for YamlFileProvider {
    async fn get_profile(&self, features: &HashSet<Hash>) -> Result<Option<Profile>, Errors> {
        let yaml = self.load_data().await?;

        let profiles: Vec<Profile> = yaml
            .get("profiles")
            .and_then(|dp| {
                let deserialized: Result<Vec<Profile>, _> = serde_yaml::from_value(dp.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for profiles: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        if let Some(target_profile) = Profile::get_similar_profile(&profiles, features) {
            Ok(Some(target_profile))
        } else {
            Ok(None)
        }
    }

    async fn save_profile(&self, profile: &Profile) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let mapping = yaml.as_mapping_mut().ok_or_else(|| {
            Errors::YamlParseError("Expected root YAML value to be a mapping.".to_string())
        })?;

        let profiles = mapping
            .entry(serde_yaml::Value::String("profiles".to_string()))
            .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()))
            .as_sequence_mut()
            .ok_or_else(|| {
                Errors::YamlParseError(
                    "Failed to get or create mutable sequence for 'profiles'.".to_string(),
                )
            })?;

        let new_profile_yaml =
            serde_yaml::to_value(&profile).map_err(|_| Errors::UnexpectedError)?;
        profiles.push(new_profile_yaml);

        self.save_data(&yaml).await
    }

    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<BasisNode>, Errors> {
        let yaml = self.load_data().await?;

        let basis_nodes: Vec<BasisNode> = yaml
            .get("basis_nodes")
            .and_then(|bn| {
                let deserialized: Result<Vec<BasisNode>, _> = serde_yaml::from_value(bn.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for basis_nodes: {:?}", err);
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
        lineage: &Lineage,
        basis_node: BasisNode,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_node =
            serde_yaml::to_value(&basis_node).map_err(|_| Errors::UnexpectedError)?;

        if let Some(basis_nodes) = yaml.get_mut("basis_nodes") {
            let sequence = basis_nodes.as_sequence_mut().ok_or_else(|| {
                Errors::YamlParseError(
                    "Failed to get mutable sequence for 'basis_nodes'.".to_string(),
                )
            })?;

            // Remove existing entry with matching lineage
            sequence.retain(|node| {
                if let Ok(existing_node) = serde_yaml::from_value::<BasisNode>(node.clone()) {
                    &existing_node.lineage != lineage
                } else {
                    true
                }
            });

            sequence.push(serialized_basis_node);
        } else {
            yaml["basis_nodes"] = serde_yaml::Value::Sequence(vec![serialized_basis_node]);
        }

        self.save_data(&yaml).await
    }

    async fn get_basis_network_by_lineage_and_subgraph_hash(
        &self,
        lineage: &Lineage,
        subgraph_hash: &Hash,
    ) -> Result<Option<BasisNetwork>, Errors> {
        let yaml = self.load_data().await?;

        let basis_networks: Vec<BasisNetwork> = yaml
            .get("basis_networks")
            .and_then(|bn| {
                let deserialized: Result<Vec<BasisNetwork>, _> = serde_yaml::from_value(bn.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for basis_networks: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        for basis_network in basis_networks {
            if &basis_network.lineage == lineage && &basis_network.subgraph_hash == subgraph_hash {
                return Ok(Some(basis_network));
            }
        }

        Ok(None)
    }

    async fn save_basis_network(
        &self,
        lineage: &Lineage,
        subgraph_hash: &Hash,
        basis_network: BasisNetwork,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_network =
            serde_yaml::to_value(&basis_network).map_err(|_| Errors::UnexpectedError)?;

        if let Some(basis_networks) = yaml.get_mut("basis_networks") {
            let sequence = basis_networks.as_sequence_mut().ok_or_else(|| {
                Errors::YamlParseError(
                    "Failed to get mutable sequence for 'basis_networks'.".to_string(),
                )
            })?;

            // Remove existing entry with matching lineage and subgraph_hash
            sequence.retain(|network| {
                if let Ok(existing_network) =
                    serde_yaml::from_value::<BasisNetwork>(network.clone())
                {
                    !(existing_network.lineage == *lineage && existing_network.subgraph_hash == *subgraph_hash)
                } else {
                    true
                }
            });

            sequence.push(serialized_basis_network);
        } else {
            yaml["basis_networks"] = serde_yaml::Value::Sequence(vec![serialized_basis_network]);
        }

        self.save_data(&yaml).await
    }

    async fn get_classification_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<Classification>, Errors> {
        let yaml = self.load_data().await?;

        let classifications: Vec<Classification> = yaml
            .get("classifications")
            .and_then(|bn| {
                let deserialized: Result<Vec<Classification>, _> = serde_yaml::from_value(bn.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for classifications: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        for classification in classifications {
            if classification.lineage == *lineage {
                return Ok(Some(classification));
            }
        }

        Ok(None)
    }

    async fn save_classification(
        &self,
        lineage: &Lineage,
        classification: Classification,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_classification =
            serde_yaml::to_value(&classification).map_err(|_| Errors::UnexpectedError)?;

        if let Some(classifications) = yaml.get_mut("classifications") {
            let sequence = classifications.as_sequence_mut().ok_or_else(|| {
                Errors::YamlParseError(
                    "Failed to get mutable sequence for 'classifications'.".to_string(),
                )
            })?;

            // Remove existing entry with matching lineage
            sequence.retain(|entry| {
                if let Ok(existing) = serde_yaml::from_value::<Classification>(entry.clone()) {
                    existing.lineage != *lineage
                } else {
                    true
                }
            });

            sequence.push(serialized_classification);
        } else {
            yaml["classifications"] = serde_yaml::Value::Sequence(vec![serialized_classification]);
        }

        self.save_data(&yaml).await
    }

    async fn get_operation_by_hash(&self, hash: &Hash) -> Result<Option<Operation>, Errors> {
        let yaml = self.load_data().await?;

        let bloom_filter = yaml
            .get("no_op")
            .and_then(|data| {
                let deserialized: Result<BloomFilter, _> = serde_yaml::from_value(data.clone());

                if let Err(ref err) = deserialized {
                    log::error!(
                        "Deserialization error for operation bloom filter: {:?}",
                        err
                    );
                }
                deserialized.ok()
            })
            .unwrap_or_else(|| BloomFilter::new(100, 7));

        if bloom_filter.contains(hash) {
            log::info!("bloom filter: operation might be a no-op");
        } else {
            log::info!("bloom filter: operation definitely not no-op");

            let operations: Vec<Operation> = yaml
                .get("operations")
                .and_then(|data| {
                    let deserialized: Result<Vec<Operation>, _> =
                        serde_yaml::from_value(data.clone());

                    if let Err(ref err) = deserialized {
                        log::error!("Deserialization error for operations: {:?}", err);
                    }
                    deserialized.ok()
                })
                .unwrap_or_else(Vec::new);

            for operation in operations {
                if &operation.hash == hash {
                    return Ok(Some(operation));
                }
            }
        }

        Ok(None)
    }

    async fn get_basis_graph_by_hash(&self, hash: &Hash) -> Result<Option<BasisGraph>, Errors> {
        let yaml = self.load_data().await?;

        let basis_graphs: Vec<BasisGraph> = yaml
            .get("basis_graphs")
            .and_then(|data| {
                let deserialized: Result<Vec<BasisGraph>, _> = serde_yaml::from_value(data.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for basis_graphs: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        for basis_graph in basis_graphs {
            if &basis_graph.hash == hash {
                return Ok(Some(basis_graph));
            }
        }

        Ok(None)
    }

    async fn save_basis_graph(
        &self,
        hash: &Hash,
        basis_graph: BasisGraph,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_graph =
            serde_yaml::to_value(&basis_graph).map_err(|_| Errors::UnexpectedError)?;

        if let Some(basis_graphs) = yaml.get_mut("basis_graphs") {
            let sequence = basis_graphs.as_sequence_mut().ok_or_else(|| {
                Errors::YamlParseError(
                    "Failed to get mutable sequence for 'basis_graphs'.".to_string(),
                )
            })?;

            sequence.retain(|entry| {
                if let Ok(existing) = serde_yaml::from_value::<BasisGraph>(entry.clone()) {
                    &existing.hash != hash
                } else {
                    true
                }
            });

            sequence.push(serialized_basis_graph);
        } else {
            yaml["basis_graphs"] = serde_yaml::Value::Sequence(vec![serialized_basis_graph]);
        }

        self.save_data(&yaml).await
    }

    async fn save_operation(&self, hash: &Hash, operation: Operation) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let mut bloom_filter = yaml
            .get("no_op")
            .and_then(|data| {
                let deserialized: Result<BloomFilter, _> = serde_yaml::from_value(data.clone());

                if let Err(ref err) = deserialized {
                    log::error!(
                        "Deserialization error for operation bloom filter: {:?}",
                        err
                    );
                }
                deserialized.ok()
            })
            .unwrap_or_else(|| BloomFilter::new(1_048_576, 7));

        if operation.is_no_op() {
            bloom_filter.add(hash);

            let serialized_bloom_filter =
                serde_yaml::to_value(bloom_filter).map_err(|_| Errors::UnexpectedError)?;
            yaml["no_op"] = serialized_bloom_filter;

            return self.save_data(&yaml).await;
        }

        let serialized_operation =
            serde_yaml::to_value(&operation).map_err(|_| Errors::UnexpectedError)?;

        if let Some(operations) = yaml.get_mut("operations") {
            operations
                .as_sequence_mut()
                .ok_or_else(|| {
                    Errors::YamlParseError(
                        "Failed to get mutable sequence for operations".to_string(),
                    )
                })?
                .push(serialized_operation);
        } else {
            yaml["operations"] = serde_yaml::Value::Sequence(vec![serialized_operation]);
        }

        self.save_data(&yaml).await
    }
}
