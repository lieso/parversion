use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::sync::RwLock as AsyncRwLock;

use crate::basis_field::BasisField;
use crate::basis_group::BasisGroup;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::classification::Classification;
use crate::document::Document;
use crate::network_relationship::NetworkRelationship;
use crate::node_relationship::NodeRelationship;
use crate::prelude::*;
use crate::provider::Provider;

/// A keyed record in a yaml section. `keys` plays the role of the composite
/// primary key that the sqlite provider uses for each table.
#[derive(Serialize, Deserialize)]
struct KeyedEntry<T> {
    keys: Vec<String>,
    data: T,
}

/// Deserializes only the key portion of an entry, ignoring `data`.
#[derive(Deserialize)]
struct EntryKeys {
    keys: Vec<String>,
}

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
        let new_yaml_str = serde_yaml::to_string(yaml)
            .map_err(|_| Errors::UnexpectedError("Serialization error".to_string()))?;
        async_fs::write(&self.file_path, new_yaml_str)
            .await
            .map_err(|_| Errors::UnexpectedError("Serialization error".to_string()))?;
        let mut cache = self.cache.write().await;
        *cache = Some(yaml.clone());
        Ok(())
    }
}

fn hash_key(hash: &Hash) -> Result<String, Errors> {
    hash.to_string()
        .ok_or_else(|| Errors::UnexpectedError("Hash is not finalized".to_string()))
}

fn basis_node_keys(basis_nodes: &[Arc<BasisNode>]) -> Vec<String> {
    let mut keys: Vec<String> = basis_nodes
        .iter()
        .map(|bn| bn.lineage.to_string())
        .collect();
    keys.sort();
    keys
}

fn sorted_pair(a: String, b: String) -> Vec<String> {
    if a <= b {
        vec![a, b]
    } else {
        vec![b, a]
    }
}

fn read_entries<T: DeserializeOwned>(
    yaml: &serde_yaml::Value,
    section: &str,
) -> Vec<KeyedEntry<T>> {
    yaml.get(section)
        .and_then(|s| s.as_sequence())
        .map(|sequence| {
            sequence
                .iter()
                .filter_map(|entry| {
                    let deserialized = serde_yaml::from_value::<KeyedEntry<T>>(entry.clone());
                    if let Err(ref err) = deserialized {
                        log::error!("Deserialization error for {}: {:?}", section, err);
                    }
                    deserialized.ok()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the data of the entry whose keys match exactly.
fn find_entry<T: DeserializeOwned>(
    yaml: &serde_yaml::Value,
    section: &str,
    keys: &[String],
) -> Option<T> {
    read_entries::<T>(yaml, section)
        .into_iter()
        .find(|entry| entry.keys.as_slice() == keys)
        .map(|entry| entry.data)
}

/// Returns the data of every entry whose keys begin with `prefix`.
fn find_entries_with_prefix<T: DeserializeOwned>(
    yaml: &serde_yaml::Value,
    section: &str,
    prefix: &[String],
) -> Vec<T> {
    read_entries::<T>(yaml, section)
        .into_iter()
        .filter(|entry| entry.keys.starts_with(prefix))
        .map(|entry| entry.data)
        .collect()
}

/// Inserts or replaces the entry with the given keys, mirroring `INSERT OR REPLACE`.
fn upsert_entry<T: Serialize>(
    yaml: &mut serde_yaml::Value,
    section: &str,
    keys: Vec<String>,
    data: T,
) -> Result<(), Errors> {
    let serialized = serde_yaml::to_value(KeyedEntry {
        keys: keys.clone(),
        data,
    })
    .map_err(|_| Errors::UnexpectedError("Serialization error".to_string()))?;

    if yaml.get(section).is_none() {
        yaml[section] = serde_yaml::Value::Sequence(Vec::new());
    }

    let sequence = yaml[section].as_sequence_mut().ok_or_else(|| {
        Errors::YamlParseError(format!("Failed to get mutable sequence for '{}'.", section))
    })?;

    sequence.retain(|entry| {
        serde_yaml::from_value::<EntryKeys>(entry.clone())
            .map(|existing| existing.keys != keys)
            .unwrap_or(true)
    });

    sequence.push(serialized);
    Ok(())
}

/// Collapses rows into the sqlite provider's convention: no rows is `None`,
/// while rows with null data (a recorded "no match") are dropped but still count.
fn collect_basis_groups(rows: Vec<Option<BasisGroup>>) -> Option<Vec<BasisGroup>> {
    if rows.is_empty() {
        None
    } else {
        Some(rows.into_iter().flatten().collect())
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

        let serialized_basis_node = serde_yaml::to_value(&basis_node)
            .map_err(|_| Errors::UnexpectedError("Serialization error".to_string()))?;

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

    async fn get_classification_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<Classification>, Errors> {
        let yaml = self.load_data().await?;

        let classifications: Vec<Classification> = yaml
            .get("classifications")
            .and_then(|bn| {
                let deserialized: Result<Vec<Classification>, _> =
                    serde_yaml::from_value(bn.clone());
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

        let serialized_classification = serde_yaml::to_value(&classification)
            .map_err(|_| Errors::UnexpectedError("Serialization error".to_string()))?;

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

    async fn get_basis_groups_by_acyclic_lineage(
        &self,
        acyclic_lineage: &Lineage,
    ) -> Result<Option<Vec<BasisGroup>>, Errors> {
        let yaml = self.load_data().await?;
        let prefix = [acyclic_lineage.to_string()];

        Ok(collect_basis_groups(find_entries_with_prefix(
            &yaml,
            "basis_groups",
            &prefix,
        )))
    }

    async fn get_basis_groups_by_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
    ) -> Result<Option<Vec<BasisGroup>>, Errors> {
        let yaml = self.load_data().await?;
        let prefix = [acyclic_lineage.to_string(), lineage.to_string()];

        Ok(collect_basis_groups(find_entries_with_prefix(
            &yaml,
            "basis_groups",
            &prefix,
        )))
    }

    async fn get_basis_groups_by_indexed_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
        indexed_lineage: &Lineage,
    ) -> Result<Option<Vec<BasisGroup>>, Errors> {
        let yaml = self.load_data().await?;
        let prefix = [
            acyclic_lineage.to_string(),
            lineage.to_string(),
            indexed_lineage.to_string(),
        ];

        Ok(collect_basis_groups(find_entries_with_prefix(
            &yaml,
            "basis_groups",
            &prefix,
        )))
    }

    async fn save_basis_group(
        &self,
        acyclic_lineage: &Lineage,
        lineage: Option<&Lineage>,
        indexed_lineage: Option<&Lineage>,
        basis_group: Option<BasisGroup>,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let keys = vec![
            acyclic_lineage.to_string(),
            lineage.map(|l| l.to_string()).unwrap_or_default(),
            indexed_lineage.map(|l| l.to_string()).unwrap_or_default(),
        ];
        upsert_entry(&mut yaml, "basis_groups", keys, basis_group)?;

        self.save_data(&yaml).await
    }

    async fn get_basis_fields_by_acyclic_subgraph_hash(
        &self,
        acyclic_subgraph_hash: &Hash,
    ) -> Result<Vec<BasisField>, Errors> {
        let yaml = self.load_data().await?;

        let basis_fields: Vec<BasisField> = yaml
            .get("basis_fields")
            .and_then(|bf| {
                let deserialized: Result<Vec<BasisField>, _> = serde_yaml::from_value(bf.clone());
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
        basis_fields: Vec<BasisField>,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let serialized_basis_fields: Vec<serde_yaml::Value> = basis_fields
            .into_iter()
            .map(|field| {
                serde_yaml::to_value(&field)
                    .map_err(|_| Errors::UnexpectedError("Serialization error".to_string()))
            })
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

    async fn save_schema_instance_document(
        &self,
        hash: &Hash,
        document: Document,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        upsert_entry(&mut yaml, "documents", vec![hash_key(hash)?], document)?;

        self.save_data(&yaml).await
    }

    async fn get_instance_document_by_schema_hash(
        &self,
        hash: &Hash,
    ) -> Result<Option<Document>, Errors> {
        let yaml = self.load_data().await?;

        Ok(find_entry(&yaml, "documents", &[hash_key(hash)?]))
    }

    async fn get_node_relationships(
        &self,
        left_basis_lineage: &Lineage,
        right_basis_lineage: &Lineage,
    ) -> Result<Option<Vec<NodeRelationship>>, Errors> {
        let yaml = self.load_data().await?;
        let keys = [
            left_basis_lineage.to_string(),
            right_basis_lineage.to_string(),
        ];

        Ok(find_entry(&yaml, "node_relationships", &keys))
    }

    async fn save_node_relationships(
        &self,
        left_basis_lineage: Lineage,
        right_basis_lineage: Lineage,
        node_relationships: Vec<NodeRelationship>,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let keys = vec![
            left_basis_lineage.to_string(),
            right_basis_lineage.to_string(),
        ];
        upsert_entry(&mut yaml, "node_relationships", keys, node_relationships)?;

        self.save_data(&yaml).await
    }

    async fn get_basis_network(
        &self,
        basis_nodes: Vec<Arc<BasisNode>>,
    ) -> Result<Option<BasisNetwork>, Errors> {
        let yaml = self.load_data().await?;

        Ok(find_entry(
            &yaml,
            "basis_networks",
            &basis_node_keys(&basis_nodes),
        ))
    }

    async fn save_basis_network(
        &self,
        basis_nodes: Vec<Arc<BasisNode>>,
        basis_network: BasisNetwork,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        upsert_entry(
            &mut yaml,
            "basis_networks",
            basis_node_keys(&basis_nodes),
            basis_network,
        )?;

        self.save_data(&yaml).await
    }

    async fn get_network_relationship(
        &self,
        left: Arc<BasisNetwork>,
        right: Arc<BasisNetwork>,
    ) -> Result<Option<NetworkRelationship>, Errors> {
        let yaml = self.load_data().await?;
        let keys = sorted_pair(left.lineage.to_string(), right.lineage.to_string());

        Ok(find_entry(&yaml, "network_relationships", &keys))
    }

    async fn save_network_relationship(
        &self,
        left: Arc<BasisNetwork>,
        right: Arc<BasisNetwork>,
        network_relationship: NetworkRelationship,
    ) -> Result<(), Errors> {
        let mut yaml = self.load_data().await?;

        let keys = sorted_pair(left.lineage.to_string(), right.lineage.to_string());
        upsert_entry(
            &mut yaml,
            "network_relationships",
            keys,
            network_relationship,
        )?;

        self.save_data(&yaml).await
    }
}
