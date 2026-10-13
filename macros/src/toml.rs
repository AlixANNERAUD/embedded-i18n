use std::{collections::HashMap, path::Path};

use ::toml::{Table, Value};

use crate::tree::{Node, flatten};

pub const EXTENSIONS: &[&str] = &["toml"];

pub fn load(path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    parse(&content)
}

fn parse(content: &str) -> Result<HashMap<String, String>, String> {
    let table = content
        .parse::<Table>()
        .map_err(|e| format!("TOML parse error: {}", e))?;
    flatten(entries("", table)?)
}

fn entries(key: &str, table: Table) -> Result<Vec<(String, Node)>, String> {
    table
        .into_iter()
        .map(|(child, value)| {
            let path = if key.is_empty() {
                child.clone()
            } else {
                format!("{}.{}", key, child)
            };
            let node = match value {
                Value::String(value) => Node::Text(value),
                Value::Table(table) => Node::Map(entries(&path, table)?),
                _ => {
                    return Err(format!(
                        "TOML parse error: expected a string or a table for key {:?}",
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
    fn parses_strings_and_tables() {
        let translations = parse(
            "greeting = \"Hello\"\n\"with.dot\" = \"Dot\"\n\n[menu]\nopen = \"Open\"\n\n[menu.settings]\ntitle = \"Settings\"\n",
        )
        .unwrap();
        assert_eq!(translations.len(), 4);
        assert_eq!(translations["greeting"], "Hello");
        assert_eq!(translations["with.dot"], "Dot");
        assert_eq!(translations["menu.open"], "Open");
        assert_eq!(translations["menu.settings.title"], "Settings");
    }

    #[test]
    fn rejects_non_string_values() {
        assert_eq!(
            parse("[menu]\ncount = 3\n").err().unwrap(),
            "TOML parse error: expected a string or a table for key \"menu.count\""
        );
    }

    #[test]
    fn reports_syntax_errors() {
        assert!(
            parse("greeting = ")
                .err()
                .unwrap()
                .starts_with("TOML parse error")
        );
    }
}
