use std::{collections::HashMap, path::Path};

use serde_json::Value;

use crate::tree::{Node, flatten};

pub const EXTENSIONS: &[&str] = &["json"];

pub fn load(path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    parse(&content)
}

fn parse(content: &str) -> Result<HashMap<String, String>, String> {
    let value =
        serde_json::from_str::<Value>(content).map_err(|e| format!("JSON parse error: {}", e))?;
    match node("", value)? {
        Node::Map(entries) => flatten(entries),
        Node::Text(_) => Err("JSON parse error: expected an object at the top level".to_string()),
    }
}

fn node(key: &str, value: Value) -> Result<Node, String> {
    match value {
        Value::String(value) => Ok(Node::Text(value)),
        Value::Object(entries) => entries
            .into_iter()
            .map(|(child, value)| {
                let path = if key.is_empty() {
                    child.clone()
                } else {
                    format!("{}.{}", key, child)
                };
                node(&path, value).map(|node| (child, node))
            })
            .collect::<Result<_, _>>()
            .map(Node::Map),
        _ => Err(format!(
            "JSON parse error: expected a string or an object for key {:?}",
            key
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flat_object() {
        let translations = parse(r#"{"greeting": "Hello", "menu.open": "Open"}"#).unwrap();
        assert_eq!(translations["greeting"], "Hello");
        assert_eq!(translations["menu.open"], "Open");
    }

    #[test]
    fn flattens_nested_objects() {
        let translations =
            parse(r#"{"menu": {"open": "Open", "settings": {"title": "Settings"}}}"#).unwrap();
        assert_eq!(translations["menu.open"], "Open");
        assert_eq!(translations["menu.settings.title"], "Settings");
    }

    #[test]
    fn rejects_non_string_values() {
        assert_eq!(
            parse(r#"{"menu": {"count": 3}}"#).err().unwrap(),
            "JSON parse error: expected a string or an object for key \"menu.count\""
        );
    }

    #[test]
    fn rejects_top_level_array() {
        assert!(parse(r#"["a"]"#).is_err());
    }
}
