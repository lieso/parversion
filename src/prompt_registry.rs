use std::path::Path;
use std::future::Future;
use std::pin::Pin;
use std::collections::HashMap;

use crate::prelude::*;

type PromptNodeName = String;

type FetchFn = Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<String, Errors>> + Send>> + Send + Sync>;

struct PromptNode {
    name: PromptNodeName,
    children: HashMap<PromptNodeName, PromptNode>,
    operations: HashMap<String, FetchFn>
}

pub struct PromptRegistry {
    root: PromptNode,
}

impl PromptRegistry {
    pub fn load(location: String) -> Result<Self, Errors> {
        let (scheme, path) = location.split_once("://")
            .ok_or_else(|| Errors::PromptRegistryError(
                format!("Missing protocol in prompts location: {}", location)
            ))?;

        match scheme {
            "file" => Self::load_from_filesystem(path),
            "http" | "https" => Err(Errors::PromptRegistryError(
                "HTTP prompt loading is not yet implemented".to_string()
            )),
            _ => Err(Errors::PromptRegistryError(
                format!("Unknown protocol '{}' in prompts location", scheme)
            )),
        }
    }

    fn load_from_filesystem(path: &str) -> Result<Self, Errors> {
        let root = Self::load_directory(Path::new(path), "root")?;
        let registry = PromptRegistry {
            root
        };

        Ok(registry)
    }

    fn load_directory(dir: &Path, name: &str) -> Result<PromptNode, Errors> {
        let mut prompt_node = PromptNode {
            name: name.to_string(),
            children: HashMap::new(),
            operations: HashMap::new(),
        };

        for entry in std::fs::read_dir(dir).map_err(|e| {
            Errors::PromptRegistryError(format!("Could not read directory '{}': {}", dir.display(), e))
        })? {
            let entry = entry.map_err(io_err)?;
            let file_type = entry.file_type().map_err(io_err)?;
            let name = entry.file_name().to_string_lossy().into_owned();

            if file_type.is_dir() {
                let child = Self::load_directory(&entry.path(), &name)?;
                prompt_node.children.insert(name, child);
            } else if file_type.is_file() {
                let file_path = entry.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("txt") { continue; }
                let stem = file_path.file_stem().unwrap().to_string_lossy().into_owned();
                let fetch: FetchFn = Box::new(move || {
                    let file_path = file_path.clone();
                    Box::pin(async move {
                        tokio::fs::read_to_string(&file_path).await.map_err(io_err)
                    })
                });
                prompt_node.operations.insert(stem, fetch);
            }
        }

        Ok(prompt_node)
    }

    pub async fn get(&self, path: &str, operation: &str) -> Result<Option<String>, Errors> {
        let mut current = &self.root;

        for segment in path.split('/').filter(|s| !s.is_empty()) {
            match current.children.get(segment) {
                Some(child) => current = child,
                None => return Ok(None),
            }
        }

        match current.operations.get(operation) {
            Some(fetch) => fetch().await.map(Some),
            None => Ok(None),
        }
    }
}

fn io_err(e: std::io::Error) -> Errors {
    Errors::PromptRegistryError(e.to_string())
}
