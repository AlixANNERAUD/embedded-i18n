use std::{collections::HashMap, path::Path};

use yaml_rust2::{Yaml, YamlLoader};

use crate::tree::{Node, flatten};

pub const EXTENSIONS: &[&str] = &["yaml", "yml"];

pub fn load(path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    parse(&content)
}

fn parse(content: &str) -> Result<HashMap<String, String>, String> {
    let documents =
        YamlLoader::load_from_str(content).map_err(|e| format!("YAML parse error: {}", e))?;
    match documents.into_iter().next() {
        None | Some(Yaml::Null) => Ok(HashMap::new()),
        Some(Yaml::Hash(hash)) => flatten(entries("", hash)?),
        Some(_) => Err("YAML parse error: expected a mapping at the top level".to_string()),
    }
}

fn entries(key: &str, hash: yaml_rust2::yaml::Hash) -> Result<Vec<(String, Node)>, String> {
    hash.into_iter()
        .map(|(child, value)| {
            let child = match child {
                Yaml::String(child) => child,
                Yaml::Integer(child) => child.to_string(),
                _ => {
                    return Err(format!(
                        "YAML parse error: expected a string key under {:?}",
                        key
                    ));
                }
            };
            let path = if key.is_empty() {
                child.clone()
            } else {
                format!("{}.{}", key, child)
            };
            let node = match value {
                Yaml::String(value) => Node::Text(value),
                Yaml::Hash(hash) => Node::Map(entries(&path, hash)?),
                _ => {
                    return Err(format!(
                        "YAML parse error: expected a string or a mapping for key {:?}",
                        path
                    ));
                }
            };
            Ok((child, node))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_mappings() {
        let translations =
            parse("greeting: Hello\nmenu:\n  open: \"Open\"\n  settings:\n    title: Settings\n")
                .unwrap();
        assert_eq!(translations.len(), 3);
        assert_eq!(translations["greeting"], "Hello");
        assert_eq!(translations["menu.open"], "Open");
        assert_eq!(translations["menu.settings.title"], "Settings");
    }

    #[test]
    fn empty_document_has_no_translations() {
        assert!(parse("").unwrap().is_empty());
    }

    #[test]
    fn rejects_non_string_values() {
        assert_eq!(
            parse("menu:\n  enabled: true\n").err().unwrap(),
            "YAML parse error: expected a string or a mapping for key \"menu.enabled\""
        );
    }

    #[test]
    fn rejects_top_level_sequence() {
        assert!(parse("- a\n- b\n").is_err());
    }
}
