use std::path::Path;
use std::collections::HashMap;

use crate::prelude::*;

pub struct PromptRegistry {
    tree: HashMap<String, HashMap<String, String>>,
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
        let root = Path::new(path);
        let mut tree: HashMap<String, HashMap<String, String>> = HashMap::new();

        for entry in std::fs::read_dir(root).map_err(|e| {
            Errors::PromptRegistryError(format!("Could not read prompts directory '{}': {}", path, e))
        })? {
            let entry = entry.map_err(io_err)?;
            if !entry.file_type().map_err(io_err)?.is_dir() { continue; }

            let dir_name = entry.file_name().to_string_lossy().into_owned();
            let mut operations: HashMap<String, String> = HashMap::new();

            for file in std::fs::read_dir(entry.path()).map_err(io_err)? {
                let file = file.map_err(io_err)?;
                let file_path = file.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("txt") { continue; }

                let operation = file_path.file_stem().unwrap().to_string_lossy().into_owned();
                let content = std::fs::read_to_string(&file_path).map_err(io_err)?;
                operations.insert(operation, content);
            }

            tree.insert(dir_name, operations);
        }

        Ok(PromptRegistry { tree })
    }
}

fn io_err(e: std::io::Error) -> Errors {
    Errors::PromptRegistryError(e.to_string())
}
