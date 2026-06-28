use std::path::Path;
use std::collections::HashMap;

use crate::prelude::*;

pub enum PromptNode {
    Leaf(String),
    Branch(HashMap<String, PromptNode>),
}

pub struct PromptRegistry {
    root: HashMap<String, PromptNode>,
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
        let root = Self::load_directory(Path::new(path))?;
        Ok(PromptRegistry { root })
    }

    fn load_directory(dir: &Path) -> Result<HashMap<String, PromptNode>, Errors> {
        let mut nodes = HashMap::new();

        for entry in std::fs::read_dir(dir).map_err(|e| {
            Errors::PromptRegistryError(format!("Could not read directory '{}': {}", dir.display(), e))
        })? {
            let entry = entry.map_err(io_err)?;
            let file_type = entry.file_type().map_err(io_err)?;
            let name = entry.file_name().to_string_lossy().into_owned();

            if file_type.is_dir() {
                let children = Self::load_directory(&entry.path())?;
                nodes.insert(name, PromptNode::Branch(children));
            } else if file_type.is_file() {
                let file_path = entry.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("txt") { continue; }
                let stem = file_path.file_stem().unwrap().to_string_lossy().into_owned();
                let content = std::fs::read_to_string(&file_path).map_err(io_err)?;
                nodes.insert(stem, PromptNode::Leaf(content));
            }
        }

        Ok(nodes)
    }

    pub fn get(&self, path: &str, operation: &str) -> Option<&str> {
        let mut current = &self.root;

        for segment in path.split('/') {
            match current.get(segment)? {
                PromptNode::Branch(children) => current = children,
                PromptNode::Leaf(_) => return None,
            }
        }

        match current.get(operation)? {
            PromptNode::Leaf(content) => Some(content.as_str()),
            PromptNode::Branch(_) => None,
        }
    }
}

fn io_err(e: std::io::Error) -> Errors {
    Errors::PromptRegistryError(e.to_string())
}
