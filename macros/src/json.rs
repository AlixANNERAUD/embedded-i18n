use std::{collections::HashMap, path::Path};

/// Backend: JSON locale files (.json)
pub const EXTENSION: &str = "json";

pub fn load(path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    serde_json::from_str::<HashMap<String, String>>(&content)
        .map_err(|e| format!("JSON parse error: {}", e))
}
