use std::collections::HashMap;

pub enum Node {
    Text(String),
    Map(Vec<(String, Node)>),
}

pub fn flatten(entries: Vec<(String, Node)>) -> Result<HashMap<String, String>, String> {
    let mut translations = HashMap::new();
    insert(&mut translations, "", entries)?;
    Ok(translations)
}

fn insert(
    translations: &mut HashMap<String, String>,
    prefix: &str,
    entries: Vec<(String, Node)>,
) -> Result<(), String> {
    for (key, node) in entries {
        let key = if prefix.is_empty() {
            key
        } else {
            format!("{}.{}", prefix, key)
        };
        match node {
            Node::Text(value) => {
                if translations.insert(key.clone(), value).is_some() {
                    return Err(format!("duplicate key {:?}", key));
                }
            }
            Node::Map(entries) => insert(translations, &key, entries)?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Node {
        Node::Text(value.to_string())
    }

    #[test]
    fn flattens_nested_maps_with_dots() {
        let translations = flatten(vec![
            ("title".to_string(), text("Title")),
            (
                "menu".to_string(),
                Node::Map(vec![
                    ("open".to_string(), text("Open")),
                    (
                        "settings".to_string(),
                        Node::Map(vec![("title".to_string(), text("Settings"))]),
                    ),
                ]),
            ),
        ])
        .unwrap();
        assert_eq!(translations.len(), 3);
        assert_eq!(translations["title"], "Title");
        assert_eq!(translations["menu.open"], "Open");
        assert_eq!(translations["menu.settings.title"], "Settings");
    }

    #[test]
    fn rejects_duplicate_flattened_keys() {
        let error = flatten(vec![
            ("menu.open".to_string(), text("Open")),
            (
                "menu".to_string(),
                Node::Map(vec![("open".to_string(), text("Ouvrir"))]),
            ),
        ])
        .err()
        .unwrap();
        assert_eq!(error, "duplicate key \"menu.open\"");
    }
}
