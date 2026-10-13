use std::path::Path;

use crate::plural::Expression;
use crate::{CONTEXT_SEPARATOR, Translations};

pub const EXTENSIONS: &[&str] = &["po"];

#[derive(Default)]
struct Entry {
    context: Option<String>,
    id: Option<String>,
    plural_id: Option<String>,
    strings: Vec<(usize, String)>,
    fuzzy: bool,
}

enum Field {
    None,
    Context,
    Id,
    PluralId,
    String,
}

pub fn load(path: &Path) -> Result<Translations, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    parse(&content)
}

fn parse(content: &str) -> Result<Translations, String> {
    let mut translations = Translations::default();
    for entry in entries(content)? {
        let Some(id) = entry.id else { continue };
        let mut strings = entry.strings;
        strings.sort_by_key(|(index, _)| *index);
        if id.is_empty() && entry.context.is_none() {
            if let Some((_, header)) = strings.first() {
                translations.plural_rule = Expression::from_header(header)
                    .map_err(|e| format!("PO parse error: {}", e))?;
            }
            continue;
        }
        if id.is_empty() || entry.fuzzy {
            continue;
        }
        let key = match entry.context {
            Some(context) => format!("{}{}{}", context, CONTEXT_SEPARATOR, id),
            None => id,
        };
        if strings
            .iter()
            .enumerate()
            .any(|(position, (index, _))| position != *index)
        {
            return Err(format!(
                "PO parse error: msgstr indexes of {:?} are not contiguous",
                key
            ));
        }
        let strings: Vec<String> = strings.into_iter().map(|(_, value)| value).collect();
        if strings.is_empty() || strings.iter().any(String::is_empty) {
            continue;
        }
        if entry.plural_id.is_some() {
            translations.plurals.insert(key.clone(), strings.clone());
        }
        translations.messages.insert(key, strings[0].clone());
    }
    Ok(translations)
}

fn entries(content: &str) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let mut entry = Entry::default();
    let mut field = Field::None;
    let mut pending_fuzzy = false;

    for (number, line) in content.lines().enumerate() {
        let line = line.trim();
        let error = |message: &str| format!("PO parse error at line {}: {}", number + 1, message);

        if line.is_empty() || line.starts_with('#') {
            if line.starts_with("#~") {
                continue;
            }
            if entry.id.is_some() {
                entries.push(std::mem::take(&mut entry));
                field = Field::None;
            }
            if is_fuzzy_flag(line) {
                pending_fuzzy = true;
            } else if line.is_empty() {
                pending_fuzzy = false;
            }
            continue;
        }

        if line.starts_with('"') {
            let value = parse_string(line).ok_or_else(|| error("invalid string"))?;
            let target = match field {
                Field::None => return Err(error("string without keyword")),
                Field::Context => entry.context.as_mut(),
                Field::Id => entry.id.as_mut(),
                Field::PluralId => entry.plural_id.as_mut(),
                Field::String => entry.strings.last_mut().map(|(_, value)| value),
            };
            if let Some(target) = target {
                target.push_str(&value);
            }
            continue;
        }

        let (keyword, value) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| error("expected a keyword followed by a string"))?;
        let value = parse_string(value.trim()).ok_or_else(|| error("invalid string"))?;

        if (keyword == "msgctxt" || keyword == "msgid") && entry.id.is_some() {
            entries.push(std::mem::take(&mut entry));
        }

        match keyword {
            "msgctxt" => {
                entry.context = Some(value);
                field = Field::Context;
            }
            "msgid" => {
                entry.id = Some(value);
                entry.fuzzy = std::mem::take(&mut pending_fuzzy);
                field = Field::Id;
            }
            "msgid_plural" => {
                entry.plural_id = Some(value);
                field = Field::PluralId;
            }
            "msgstr" => {
                entry.strings.push((0, value));
                field = Field::String;
            }
            _ => {
                let index = keyword
                    .strip_prefix("msgstr[")
                    .and_then(|rest| rest.strip_suffix(']'))
                    .and_then(|index| index.parse::<usize>().ok())
                    .ok_or_else(|| error(&format!("unknown keyword {:?}", keyword)))?;
                entry.strings.push((index, value));
                field = Field::String;
            }
        }
    }

    if entry.id.is_some() {
        entries.push(entry);
    }

    Ok(entries)
}

fn is_fuzzy_flag(line: &str) -> bool {
    line.strip_prefix("#,")
        .is_some_and(|flags| flags.split(',').any(|flag| flag.trim() == "fuzzy"))
}

fn parse_string(s: &str) -> Option<String> {
    let inner = s.strip_prefix('"')?.strip_suffix('"')?;
    Some(unescape_po(inner))
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
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
        assert_eq!(map.get("Goodbye").unwrap(), "Au revoir");
    }

    #[test]
    fn test_po_with_headers() {
        let content = b"msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n\"MIME-Version: 1.0\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
    }

    #[test]
    fn test_po_multi_line() {
        let content =
            b"msgid \"\"\n\"Hello \"\n\"World\"\nmsgstr \"\"\n\"Bonjour \"\n\"le monde\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.get("Hello World").unwrap(), "Bonjour le monde");
    }

    #[test]
    fn test_po_with_plural() {
        let content = b"msgid \"file\"\nmsgid_plural \"files\"\nmsgstr[0] \"fichier\"\nmsgstr[1] \"fichiers\"\n\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("file").unwrap(), "fichier");
    }

    #[test]
    fn test_po_escaped_chars() {
        let content = b"msgid \"Line \\\"break\\\"\"\nmsgstr \"Saut de\\nligne\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("Line \"break\"").unwrap(), "Saut de\nligne");
    }

    #[test]
    fn test_po_comments_are_skipped() {
        let content = b"# This is a comment\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
    }

    #[test]
    fn test_po_empty_translation_skipped() {
        let content = b"msgid \"Hello\"\nmsgstr \"\"\n\nmsgid \"World\"\nmsgstr \"Monde\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("World").unwrap(), "Monde");
    }

    #[test]
    fn test_po_fuzzy_entries_are_skipped() {
        let content =
            b"#, fuzzy\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n\nmsgid \"World\"\nmsgstr \"Monde\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("World").unwrap(), "Monde");
    }

    #[test]
    fn test_po_combined_fuzzy_flags_are_skipped() {
        let content = b"#: src/main.rs:1\n#, c-format, fuzzy\n#| msgid \"Helo\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert!(map.is_empty());
    }

    #[test]
    fn test_po_fuzzy_does_not_leak_to_next_entry() {
        let content =
            b"#, fuzzy\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\nmsgid \"World\"\nmsgstr \"Monde\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("World").unwrap(), "Monde");
    }

    #[test]
    fn test_po_other_flags_are_kept() {
        let content = b"#, c-format, no-fuzzy-ish\n# fuzzy\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
    }

    #[test]
    fn test_po_fuzzy_header_is_ignored() {
        let content = b"#, fuzzy\nmsgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        let map = load(f.path()).unwrap().messages;
        assert_eq!(map.get("Hello").unwrap(), "Bonjour");
    }

    #[test]
    fn context_is_part_of_the_key() {
        let map = parse(
            "msgctxt \"menu\"\nmsgid \"Open\"\nmsgstr \"Ouvrir\"\n\nmsgctxt \"state\"\nmsgid \"Open\"\nmsgstr \"Ouvert\"\n\nmsgid \"Open\"\nmsgstr \"Ouvre\"\n",
        )
        .unwrap().messages;
        assert_eq!(map.len(), 3);
        assert_eq!(map["menu\u{4}Open"], "Ouvrir");
        assert_eq!(map["state\u{4}Open"], "Ouvert");
        assert_eq!(map["Open"], "Ouvre");
    }

    #[test]
    fn multi_line_context() {
        let map = parse("msgctxt \"\"\n\"main \"\n\"menu\"\nmsgid \"Open\"\nmsgstr \"Ouvrir\"\n")
            .unwrap()
            .messages;
        assert_eq!(map["main menu\u{4}Open"], "Ouvrir");
    }

    #[test]
    fn context_starts_a_new_entry() {
        let map = parse(
            "msgid \"Hello\"\nmsgstr \"Bonjour\"\nmsgctxt \"menu\"\nmsgid \"Open\"\nmsgstr \"Ouvrir\"\n",
        )
        .unwrap().messages;
        assert_eq!(map["Hello"], "Bonjour");
        assert_eq!(map["menu\u{4}Open"], "Ouvrir");
    }

    #[test]
    fn obsolete_entries_are_skipped() {
        let map =
            parse("#~ msgid \"Old\"\n#~ msgstr \"Vieux\"\nmsgid \"New\"\nmsgstr \"Nouveau\"\n")
                .unwrap()
                .messages;
        assert_eq!(map.len(), 1);
        assert_eq!(map["New"], "Nouveau");
    }

    #[test]
    fn reports_unknown_keyword() {
        assert_eq!(
            parse("msgid \"a\"\nmsgfoo \"b\"\n").err().unwrap(),
            "PO parse error at line 2: unknown keyword \"msgfoo\""
        );
    }

    #[test]
    fn reports_unterminated_string() {
        assert!(parse("msgid \"a\nmsgstr \"b\"\n").is_err());
    }

    #[test]
    fn plural_entries_keep_every_form() {
        let translations = parse(
            "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=3; plural=(n==1 ? 0 : n==2 ? 1 : 2);\\n\"\n\nmsgid \"file\"\nmsgid_plural \"files\"\nmsgstr[0] \"one\"\nmsgstr[2] \"many\"\nmsgstr[1] \"two\"\n",
        )
        .unwrap();
        assert_eq!(translations.plurals["file"], ["one", "two", "many"]);
        assert_eq!(translations.messages["file"], "one");
        assert_eq!(translations.plural_rule.unwrap().evaluate(2), 1);
    }

    #[test]
    fn plural_entries_with_context() {
        let translations = parse(
            "msgctxt \"inbox\"\nmsgid \"file\"\nmsgid_plural \"files\"\nmsgstr[0] \"fichier\"\nmsgstr[1] \"fichiers\"\n",
        )
        .unwrap();
        assert_eq!(
            translations.plurals["inbox\u{4}file"],
            ["fichier", "fichiers"]
        );
        assert!(translations.plural_rule.is_none());
    }

    #[test]
    fn partially_translated_plurals_are_skipped() {
        let translations = parse(
            "msgid \"file\"\nmsgid_plural \"files\"\nmsgstr[0] \"fichier\"\nmsgstr[1] \"\"\n",
        )
        .unwrap();
        assert!(translations.plurals.is_empty());
        assert!(translations.messages.is_empty());
    }

    #[test]
    fn reports_missing_plural_index() {
        assert!(
            parse("msgid \"file\"\nmsgid_plural \"files\"\nmsgstr[0] \"a\"\nmsgstr[2] \"b\"\n")
                .err()
                .unwrap()
                .contains("not contiguous")
        );
    }

    #[test]
    fn reports_invalid_plural_forms_header() {
        assert!(
            parse("msgid \"\"\nmsgstr \"Plural-Forms: nplurals=2; plural=n ==;\\n\"\n")
                .err()
                .unwrap()
                .starts_with("PO parse error: invalid plural expression")
        );
    }
}
