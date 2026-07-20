use async_trait::async_trait;
use rusqlite::params;
use std::sync::{Arc, Mutex};
use tokio::task;

use crate::basis_field::BasisField;
use crate::basis_graph::BasisGraph;
use crate::basis_group::BasisGroup;
use crate::classification::Classification;
use crate::basis_network::BasisNetwork;
use crate::basis_node::BasisNode;
use crate::translation_node::TranslationNode;
use crate::translation_network::TranslationNetwork;
use crate::operation::Operation;
use crate::prelude::*;
use crate::provider::Provider;
use crate::document::Document;

#[cfg(feature = "sqlite-provider")]
pub struct SqliteProvider {
    connection: Arc<Mutex<rusqlite::Connection>>,
}

#[cfg(feature = "sqlite-provider")]
impl SqliteProvider {
    pub fn new(file_path: &str) -> Result<Self, Errors> {
        let conn = rusqlite::Connection::open(file_path)
            .map_err(|e| Errors::ProviderError(e.to_string()))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS documents (
                 hash         TEXT PRIMARY KEY,
                 data         TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS classifications (
                 lineage_hash TEXT PRIMARY KEY,
                 data         TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS basis_fields (
                 acyclic_subgraph_hash  TEXT PRIMARY KEY,
                 data                   TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS basis_nodes (
                 lineage_hash TEXT PRIMARY KEY,
                 data         TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS basis_networks (
                 basis_lineages TEXT NOT NULL PRIMARY KEY,
                 data           TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS basis_graphs (
                 hash TEXT PRIMARY KEY,
                 data TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS basis_groups (
                 acyclic_lineage_hash  TEXT NOT NULL,
                 lineage_hash          TEXT NOT NULL DEFAULT '',
                 indexed_lineage_hash  TEXT NOT NULL DEFAULT '',
                 data                  TEXT NOT NULL,
                 PRIMARY KEY (acyclic_lineage_hash, lineage_hash, indexed_lineage_hash)
             );
             CREATE TABLE IF NOT EXISTS translation_nodes (
                 lineage_from       TEXT NOT NULL DEFAULT '',
                 lineage_to         TEXT NOT NULL DEFAULT '',
                 data               TEXT,
                 PRIMARY KEY (lineage_from, lineage_to)
             );
             CREATE TABLE IF NOT EXISTS translation_networks (
                 lineage_from       TEXT NOT NULL DEFAULT '',
                 lineage_to         TEXT NOT NULL DEFAULT '',
                 data               TEXT,
                 PRIMARY KEY (lineage_from, lineage_to)
             );
             ",
        )
        .map_err(|e| Errors::ProviderError(e.to_string()))?;

        Ok(Self {
            connection: Arc::new(Mutex::new(conn)),
        })
    }
}

fn lock_err() -> Errors {
    Errors::ProviderError("sqlite connection mutex poisoned".to_string())
}

fn db_err(e: impl std::fmt::Display) -> Errors {
    Errors::ProviderError(e.to_string())
}

fn serialize<T: serde::Serialize>(value: &T) -> Result<String, Errors> {
    serde_json::to_string(value).map_err(|e| db_err(e))
}

fn deserialize<T: serde::de::DeserializeOwned>(data: String) -> Result<T, Errors> {
    serde_json::from_str(&data).map_err(|e| db_err(e))
}

#[cfg(feature = "sqlite-provider")]
#[async_trait]
impl Provider for SqliteProvider {
    async fn get_basis_node_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<BasisNode>, Errors> {
        let conn = self.connection.clone();
        let key = lineage.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            match conn.query_row(
                "SELECT data FROM basis_nodes WHERE lineage_hash = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            ) {
                Ok(data) => deserialize(data).map(Some),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(db_err(e)),
            }
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_basis_node(
        &self,
        lineage: &Lineage,
        basis_node: BasisNode,
    ) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let key = lineage.to_string();
        let data = serialize(&basis_node)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            conn.execute(
                "INSERT OR REPLACE INTO basis_nodes (lineage_hash, data) VALUES (?1, ?2)",
                params![key, data],
            )
            .map_err(|e| db_err(e))?;
            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_basis_network_by_basis_lineages(
        &self,
        basis_lineages: &Hash,
    ) -> Result<Option<BasisNetwork>, Errors> {
        let conn = self.connection.clone();
        let key = basis_lineages.to_string().unwrap();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            match conn.query_row(
                "SELECT data FROM basis_networks WHERE basis_lineages = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            ) {
                Ok(data) => deserialize(data).map(Some),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(db_err(e)),
            }
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_basis_network(
        &self,
        basis_network: BasisNetwork,
    ) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let key = basis_network.basis_lineages.to_string().ok_or(Errors::UnexpectedError)?;
        let data = serialize(&basis_network)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            conn.execute(
                "INSERT OR REPLACE INTO basis_networks (basis_lineages, data) VALUES (?1, ?2)",
                params![key, data],
            )
            .map_err(|e| db_err(e))?;
            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_classification_by_lineage(
        &self,
        lineage: &Lineage,
    ) -> Result<Option<Classification>, Errors> {
        let conn = self.connection.clone();
        let key = lineage.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            match conn.query_row(
                "SELECT data FROM classifications WHERE lineage_hash = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            ) {
                Ok(data) => deserialize(data).map(Some),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(db_err(e)),
            }
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_classification(
        &self,
        lineage: &Lineage,
        classification: Classification,
    ) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let key = lineage.to_string();
        let data = serialize(&classification)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            conn.execute(
                "INSERT OR REPLACE INTO classifications (lineage_hash, data) VALUES (?1, ?2)",
                params![key, data],
            )
            .map_err(|e| db_err(e))?;
            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_basis_graph_by_hash(&self, hash: &Hash) -> Result<Option<BasisGraph>, Errors> {
        let conn = self.connection.clone();
        let key = hash.to_string().ok_or(Errors::UnexpectedError)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            match conn.query_row(
                "SELECT data FROM basis_graphs WHERE hash = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            ) {
                Ok(data) => deserialize(data).map(Some),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(db_err(e)),
            }
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_basis_graph(&self, hash: &Hash, basis_graph: BasisGraph) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let key = hash.to_string().ok_or(Errors::UnexpectedError)?;
        let data = serialize(&basis_graph)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            conn.execute(
                "INSERT OR REPLACE INTO basis_graphs (hash, data) VALUES (?1, ?2)",
                params![key, data],
            )
            .map_err(|e| db_err(e))?;
            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_basis_groups_by_acyclic_lineage(
        &self,
        acyclic_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        let conn = self.connection.clone();
        let key = acyclic_lineage.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            let mut stmt = conn
                .prepare("SELECT data FROM basis_groups WHERE acyclic_lineage_hash = ?1")
                .map_err(|e| db_err(e))?;

            let rows: Vec<String> = stmt
                .query_map(params![key], |row| row.get::<_, String>(0))
                .map_err(|e| db_err(e))?
                .collect::<rusqlite::Result<_>>()
                .map_err(|e| db_err(e))?;

            rows.into_iter().map(deserialize).collect()
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_basis_groups_by_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        let conn = self.connection.clone();
        let acyclic_key = acyclic_lineage.to_string();
        let lineage_key = lineage.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            let mut stmt = conn
                .prepare(
                    "SELECT data FROM basis_groups
                     WHERE acyclic_lineage_hash = ?1 AND lineage_hash = ?2",
                )
                .map_err(|e| db_err(e))?;

            let rows: Vec<String> = stmt
                .query_map(params![acyclic_key, lineage_key], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| db_err(e))?
                .collect::<rusqlite::Result<_>>()
                .map_err(|e| db_err(e))?;

            rows.into_iter().map(deserialize).collect()
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_basis_groups_by_indexed_lineage(
        &self,
        acyclic_lineage: &Lineage,
        lineage: &Lineage,
        indexed_lineage: &Lineage,
    ) -> Result<Vec<BasisGroup>, Errors> {
        let conn = self.connection.clone();
        let acyclic_key = acyclic_lineage.to_string();
        let lineage_key = lineage.to_string();
        let indexed_key = indexed_lineage.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            let mut stmt = conn
                .prepare(
                    "SELECT data FROM basis_groups
                     WHERE acyclic_lineage_hash = ?1
                       AND lineage_hash = ?2
                       AND indexed_lineage_hash = ?3",
                )
                .map_err(|e| db_err(e))?;

            let rows: Vec<String> = stmt
                .query_map(params![acyclic_key, lineage_key, indexed_key], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| db_err(e))?
                .collect::<rusqlite::Result<_>>()
                .map_err(|e| db_err(e))?;

            rows.into_iter().map(deserialize).collect()
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_basis_group(
        &self,
        _acyclic_lineage: &Lineage,
        _lineage: Option<&Lineage>,
        _indexed_lineage: Option<&Lineage>,
        basis_group: BasisGroup,
    ) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let acyclic_key = basis_group.acyclic_lineage.to_string();
        let lineage_key = basis_group
            .lineage
            .as_ref()
            .map(|l| l.to_string())
            .unwrap_or_default();
        let indexed_key = basis_group
            .indexed_lineage
            .as_ref()
            .map(|l| l.to_string())
            .unwrap_or_default();
        let data = serialize(&basis_group)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            conn.execute(
                "INSERT OR REPLACE INTO basis_groups
                 (acyclic_lineage_hash, lineage_hash, indexed_lineage_hash, data)
                 VALUES (?1, ?2, ?3, ?4)",
                params![acyclic_key, lineage_key, indexed_key, data],
            )
            .map_err(|e| db_err(e))?;
            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_basis_fields_by_acyclic_subgraph_hash(
        &self,
        acyclic_subgraph_hash: &Hash
    ) -> Result<Vec<BasisField>, Errors> {
        let conn = self.connection.clone();
        let key = acyclic_subgraph_hash.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            match conn.query_row(
                "SELECT data FROM basis_fields
                 WHERE acyclic_subgraph_hash = ?1",
                 params![key],
                 |row| row.get::<_, String>(0),
            ) {
                Ok(data) => deserialize(data),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Vec::new()),
                Err(e) => Err(db_err(e)),
            }
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_basis_fields(
        &self,
        acyclic_subgraph_hash: &Hash,
        basis_fields: Vec<BasisField>
    ) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let key = acyclic_subgraph_hash.to_string();
        let data = serialize(&basis_fields)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            conn.execute(
                "INSERT OR REPLACE INTO basis_fields
                (acyclic_subgraph_hash, data)
                VALUES (?1, ?2)",
                params![key, data]
            )
            .map_err(|e| db_err(e))?;

            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

		async fn save_schema_instance_document(
				&self,
				hash: &Hash,
				document: Document
		) -> Result<(), Errors> {
				let conn = self.connection.clone();
				let key = hash.to_string();
				let data = serialize(&document)?;

				task::spawn_blocking(move || {
						let conn = conn.lock().map_err(|_| lock_err())?;
						conn.execute(
								"INSERT OR REPLACE INTO documents
				 (hash, data)
				 VALUES (?1, ?2)",
				params![key, data]
			)
			.map_err(|e| db_err(e))?;

								Ok(())
				})
				.await
						.map_err(|_| Errors::UnexpectedError)?
		}

		async fn get_instance_document_by_schema_hash(
				&self,
				hash: &Hash
		) -> Result<Option<Document>, Errors> {
				let conn = self.connection.clone();
				let key = hash.to_string();

				task::spawn_blocking(move || {
						let conn = conn.lock().map_err(|_| lock_err())?;
						match conn.query_row(
								"SELECT data FROM documents
             WHERE hash = ?1",
             params![key],
             |row| row.get::<_, String>(0),
        ) {
          Ok(data) => {
            let doc = deserialize(data)?;
            Ok(Some(doc))
          },
          Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
          Err(e) => Err(db_err(e)),
        }
      })
      .await
      .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_translation_node_by_lineages(
        &self,
        lineage_from: &Lineage,
        lineage_to: &Lineage
    ) -> Result<Option<Option<TranslationNode>>, Errors> {
        let conn = self.connection.clone();
        let key1 = lineage_from.to_string();
        let key2 = lineage_to.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            match conn.query_row(
                "SELECT data FROM translation_nodes WHERE lineage_from = ?1 AND lineage_to = ?2",
                params![key1, key2],
                |row| row.get::<_, Option<String>>(0),
            ) {
                Ok(Some(data)) => deserialize(data).map(Some),
                Ok(None) => Ok(Some(None)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(db_err(e)),
            }
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_translation_node(
        &self,
        lineages: (Lineage, Lineage),
        translation_node: Option<TranslationNode>
    ) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let key1 = lineages.0.to_string();
        let key2 = lineages.1.to_string();

        let data = serialize(&translation_node)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;

            conn.execute(
                "INSERT OR REPLACE INTO translation_nodes (lineage_from, lineage_to, data) VALUES (?1, ?2, ?3)",
                params![key1, key2, data],
            )
            .map_err(|e| db_err(e))?;

            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn get_translation_network_by_lineages(
        &self,
        lineage_from: &Lineage,
        lineage_to: &Lineage
    ) -> Result<Option<Option<TranslationNetwork>>, Errors> {
        let conn = self.connection.clone();
        let key1 = lineage_from.to_string();
        let key2 = lineage_to.to_string();

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;
            match conn.query_row(
                "SELECT data FROM translation_networks WHERE lineage_from = ?1 AND lineage_to = ?2",
                params![key1, key2],
                |row| row.get::<_, Option<String>>(0),
            ) {
                Ok(Some(data)) => deserialize(data).map(Some),
                Ok(None) => Ok(Some(None)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(db_err(e)),
            }
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }

    async fn save_translation_network(
        &self,
        lineages: (Lineage, Lineage),
        translation_network: Option<TranslationNetwork>
    ) -> Result<(), Errors> {
        let conn = self.connection.clone();
        let key1 = lineages.0.to_string();
        let key2 = lineages.1.to_string();

        let data = serialize(&translation_network)?;

        task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| lock_err())?;

            conn.execute(
                "INSERT OR REPLACE INTO translation_networks (lineage_from, lineage_to, data) VALUES (?1, ?2, ?3)",
                params![key1, key2, data],
            )
            .map_err(|e| db_err(e))?;

            Ok(())
        })
        .await
        .map_err(|_| Errors::UnexpectedError)?
    }
}
