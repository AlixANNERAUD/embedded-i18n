use std::{collections::HashMap, path::Path};

/// Backend: GNU gettext PO locale files (.po)
pub const EXTENSION: &str = "po";

pub fn load(path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut translations = HashMap::new();
    let mut current_msgid = String::new();
    let mut current_msgstr = String::new();
    let mut in_msgid = false;
    let mut in_msgstr = false;
    let mut in_header = false;

    for line in content.lines() {
        let line = line.trim();

        // Blank lines and comments flush the current entry and are then skipped
        if line.is_empty() || line.starts_with('#') {
            flush_and_clear(
                &mut translations,
                &mut current_msgid,
                &mut current_msgstr,
                &mut in_header,
                &mut in_msgid,
                &mut in_msgstr,
            );
            continue;
        }

        // msgid "..."
        if let Some(value) = line.strip_prefix("msgid ") {
            flush_and_clear(
                &mut translations,
                &mut current_msgid,
                &mut current_msgstr,
                &mut in_header,
                &mut in_msgid,
                &mut in_msgstr,
            );
            current_msgid = parse_po_string(value);
            in_msgid = true;
            in_msgstr = false;
            if current_msgid.is_empty() {
                in_header = true;
            }
            continue;
        }

        // msgstr "..." (singular)
        if let Some(value) = line.strip_prefix("msgstr ") {
            in_msgid = false;
            in_msgstr = true;
            current_msgstr = parse_po_string(value);
            continue;
        }

        // msgstr[0] "..." — take the singular plural form
        if let Some(value) = line.strip_prefix("msgstr[0] ") {
            in_msgid = false;
            in_msgstr = true;
            current_msgstr = parse_po_string(value);
            continue;
        }

        // msgstr[N] for N>0 — skip other plural forms
        if line.starts_with("msgstr[") {
            continue;
        }

        // msgid_plural — skip (we handle msgstr[0] above)
        if line.starts_with("msgid_plural") {
            continue;
        }

        // Continuation string line: "..." continuation of previous msgid or msgstr
        if line.len() >= 2 && line.starts_with('"') && line.ends_with('"') {
            let value = unescape_po(&line[1..line.len() - 1]);
            if in_msgid {
                if current_msgid.is_empty() && !value.is_empty() {
                    in_header = false;
                }
                current_msgid.push_str(&value);
            } else if in_msgstr {
                current_msgstr.push_str(&value);
            }
        }
    }

    // Flush the final entry
    flush_and_clear(
        &mut translations,
        &mut current_msgid,
        &mut current_msgstr,
        &mut in_header,
        &mut in_msgid,
        &mut in_msgstr,
    );

    Ok(translations)
}

fn parse_po_string(s: &str) -> String {
    let s = s.trim();
    if s.len() < 2 {
        return String::new();
    }
    let inner = &s[1..s.len() - 1]; // strip surrounding quotes
    unescape_po(inner)
}

fn unescape_po(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn flush_and_clear(
    translations: &mut HashMap<String, String>,
    current_msgid: &mut String,
    current_msgstr: &mut String,
    in_header: &mut bool,
    in_msgid: &mut bool,
    in_msgstr: &mut bool,
) {
    if (*in_msgid || *in_msgstr)
        && !current_msgid.is_empty()
        && !*in_header
        && !current_msgstr.is_empty()
    {
        translations.insert(current_msgid.clone(), current_msgstr.clone());
    }
    current_msgid.clear();
    current_msgstr.clear();
    *in_header = false;
    *in_msgid = false;
    *in_msgstr = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_basic_po_parse() {
        let content =
            b"msgid \"Hello\"\nmsgstr \"Bonjour\"\n\nmsgid \"Goodbye\"\nmsgstr \"Au revoir\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
        assert_eq!(map.get("Goodbye").unwrap(), "Au revoir");
    }

    #[test]
    fn test_po_with_headers() {
        let content = b"msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n\"MIME-Version: 1.0\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
    }

    #[test]
    fn test_po_multi_line() {
        let content =
            b"msgid \"\"\n\"Hello \"\n\"World\"\nmsgstr \"\"\n\"Bonjour \"\n\"le monde\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.get("Hello World").unwrap(), "Bonjour le monde");
    }

    #[test]
    fn test_po_with_plural() {
        let content = b"msgid \"file\"\nmsgid_plural \"files\"\nmsgstr[0] \"fichier\"\nmsgstr[1] \"fichiers\"\n\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("file").unwrap(), "fichier");
    }

    #[test]
    fn test_po_escaped_chars() {
        let content = b"msgid \"Line \\\"break\\\"\"\nmsgstr \"Saut de\\nligne\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("Line \"break\"").unwrap(), "Saut de\nligne");
    }

    #[test]
    fn test_po_comments_are_skipped() {
        let content = b"# This is a comment\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
    }

    #[test]
    fn test_po_empty_translation_skipped() {
        let content = b"msgid \"Hello\"\nmsgstr \"\"\n\nmsgid \"World\"\nmsgstr \"Monde\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("World").unwrap(), "Monde");
    }

    #[test]
    fn test_po_fuzzy_flag() {
        let content = b"# fuzzy\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap();
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
    }
}
