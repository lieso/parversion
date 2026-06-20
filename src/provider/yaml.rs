use async_trait::async_trait;
use serde_yaml;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::sync::RwLock as AsyncRwLock;

use crate::basis_graph::BasisGraph;
use crate::classification::Classification;
use crate::basis_group::BasisGroup;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::basis_field::BasisField;
use crate::bloom_filter::BloomFilter;
use crate::operation::Operation;
use crate::prelude::*;
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

    async fn get_basis_groups_by_acyclic_lineage(
        &self,
        acyclic_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        let yaml = self.load_data().await?;

        let basis_groups: Vec<BasisGroup> = yaml
            .get("basis_groups")
            .and_then(|bg| {
                let deserialized: Result<Vec<BasisGroup>, _> =
                    serde_yaml::from_value(bg.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for basis_groups: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        Ok(basis_groups
            .into_iter()
            .filter(|bg| &bg.acyclic_lineage == acyclic_lineage)
            .collect())
    }

    async fn get_basis_groups_by_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        let yaml = self.load_data().await?;

        let basis_groups: Vec<BasisGroup> = yaml
            .get("basis_groups")
            .and_then(|bg| {
                let deserialized: Result<Vec<BasisGroup>, _> =
                    serde_yaml::from_value(bg.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for basis_groups: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        Ok(basis_groups
            .into_iter()
            .filter(|bg| &bg.acyclic_lineage == acyclic_lineage
                && bg.lineage.as_ref() == Some(lineage))
            .collect())
    }

    async fn get_basis_groups_by_indexed_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
        indexed_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        let yaml = self.load_data().await?;

        let basis_groups: Vec<BasisGroup> = yaml
            .get("basis_groups")
            .and_then(|bg| {
                let deserialized: Result<Vec<BasisGroup>, _> =
                    serde_yaml::from_value(bg.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for basis_groups: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        Ok(basis_groups
            .into_iter()
            .filter(|bg| &bg.acyclic_lineage == acyclic_lineage
                && bg.lineage.as_ref() == Some(lineage)
                && bg.indexed_lineage.as_ref() == Some(indexed_lineage))
            .collect())
    }

    async fn save_basis_group(
        &self,
        acyclic_lineage: &Lineage,
        lineage: Option<&Lineage>,
        indexed_lineage: Option<&Lineage>,
        basis_group: BasisGroup,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_group =
            serde_yaml::to_value(&basis_group).map_err(|_| Errors::UnexpectedError)?;

        if let Some(basis_groups) = yaml.get_mut("basis_groups") {
            let sequence = basis_groups.as_sequence_mut().ok_or_else(|| {
                Errors::YamlParseError(
                    "Failed to get mutable sequence for 'basis_groups'.".to_string(),
                )
            })?;

            sequence.retain(|entry| {
                if let Ok(existing) = serde_yaml::from_value::<BasisGroup>(entry.clone()) {
                    !(existing.acyclic_lineage == *acyclic_lineage
                        && existing.lineage.as_ref().map(|l| l) == lineage
                        && existing.indexed_lineage.as_ref().map(|l| l) == indexed_lineage)
                } else {
                    true
                }
            });

            sequence.push(serialized_basis_group);
        } else {
            yaml["basis_groups"] = serde_yaml::Value::Sequence(vec![serialized_basis_group]);
        }

        self.save_data(&yaml).await
    }

    async fn get_basis_fields_by_acyclic_subgraph_hash(
        &self,
        acyclic_subgraph_hash: &Hash
    ) -> Result<Vec<BasisField>, Errors> {
        let yaml = self.load_data().await?;

        let basis_fields: Vec<BasisField> = yaml
            .get("basis_fields")
            .and_then(|bf| {
                let deserialized: Result<Vec<BasisField>, _> =
                    serde_yaml::from_value(bf.clone());
                if let Err(ref err) = deserialized {
                    log::error!("Deserialization error for basis_fields: {:?}", err);
                }
                deserialized.ok()
            })
            .unwrap_or_else(Vec::new);

        Ok(basis_fields
            .into_iter()
            .filter(|bf| &bf.acyclic_subgraph_hash == acyclic_subgraph_hash)
            .collect())
    }

    async fn save_basis_fields(
        &self,
        acyclic_subgraph_hash: &Hash,
        basis_fields: Vec<BasisField>
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_fields: Vec<serde_yaml::Value> = basis_fields
            .into_iter()
            .map(|field| serde_yaml::to_value(&field).map_err(|_| Errors::UnexpectedError))
            .collect::<Result<_, _>>()?;

        if let Some(existing_basis_fields) = yaml.get_mut("basis_fields") {
            let sequence = existing_basis_fields.as_sequence_mut().ok_or_else(|| {
                Errors::YamlParseError(
                    "Failed to get mutable sequence for 'basis_fields'.".to_string(),
                )
            })?;

            sequence.retain(|entry| {
                if let Ok(existing) = serde_yaml::from_value::<BasisField>(entry.clone()) {
                    existing.acyclic_subgraph_hash != *acyclic_subgraph_hash
                } else {
                    true
                }
            });

            sequence.extend(serialized_basis_fields);
        } else {
            yaml["basis_fields"] = serde_yaml::Value::Sequence(serialized_basis_fields);
        }

        self.save_data(&yaml).await
    }
}
