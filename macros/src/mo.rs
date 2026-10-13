use std::{collections::HashMap, path::Path};

pub const EXTENSIONS: &[&str] = &["mo"];

const MAGIC: u32 = 0x950412de;

pub fn load(path: &Path) -> Result<HashMap<String, String>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    parse(&bytes)
}

fn parse(bytes: &[u8]) -> Result<HashMap<String, String>, String> {
    let word = |offset: usize, big_endian: bool| -> Result<u32, String> {
        let slice: [u8; 4] = bytes
            .get(offset..offset + 4)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| "MO parse error: unexpected end of file".to_string())?;
        Ok(if big_endian {
            u32::from_be_bytes(slice)
        } else {
            u32::from_le_bytes(slice)
        })
    };

    let big_endian = match word(0, false)? {
        MAGIC => false,
        value if value.swap_bytes() == MAGIC => true,
        _ => return Err("MO parse error: invalid magic number".to_string()),
    };

    if word(4, big_endian)? >> 16 > 1 {
        return Err("MO parse error: unsupported revision".to_string());
    }

    let count = word(8, big_endian)? as usize;
    let originals = word(12, big_endian)? as usize;
    let translations_offset = word(16, big_endian)? as usize;

    let string = |table: usize, index: usize| -> Result<&str, String> {
        let length = word(table + 8 * index, big_endian)? as usize;
        let offset = word(table + 8 * index + 4, big_endian)? as usize;
        let slice = bytes
            .get(offset..offset + length)
            .ok_or_else(|| "MO parse error: string out of bounds".to_string())?;
        std::str::from_utf8(slice).map_err(|_| "MO parse error: string is not UTF-8".to_string())
    };

    let mut translations = HashMap::new();

    for index in 0..count {
        let original = string(originals, index)?;
        if original.is_empty() {
            continue;
        }
        let key = original.split('\0').next().unwrap_or_default();
        let value = string(translations_offset, index)?
            .split('\0')
            .next()
            .unwrap_or_default();
        if !value.is_empty() {
            translations.insert(key.to_string(), value.to_string());
        }
    }

    Ok(translations)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(entries: &[(&str, &str)], big_endian: bool) -> Vec<u8> {
        let word = |value: u32| {
            if big_endian {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            }
        };
        let count = entries.len() as u32;
        let originals = 28;
        let translations = originals + 8 * count;
        let mut strings_offset = translations + 8 * count;
        let mut header = Vec::new();
        for value in [MAGIC, 0, count, originals, translations, 0, 0] {
            header.extend_from_slice(&word(value));
        }
        let mut tables = Vec::new();
        let mut strings = Vec::new();
        for column in 0..2 {
            for (original, translation) in entries {
                let value = if column == 0 { original } else { translation };
                tables.extend_from_slice(&word(value.len() as u32));
                tables.extend_from_slice(&word(strings_offset));
                strings.extend_from_slice(value.as_bytes());
                strings.push(0);
                strings_offset += value.len() as u32 + 1;
            }
        }
        [header, tables, strings].concat()
    }

    #[test]
    fn parses_little_endian() {
        let bytes = build(
            &[
                ("", "Content-Type: text/plain; charset=UTF-8\n"),
                ("Hello", "Bonjour"),
                ("file\0files", "fichier\0fichiers"),
                ("Empty", ""),
            ],
            false,
        );
        let translations = parse(&bytes).unwrap();
        assert_eq!(translations.len(), 2);
        assert_eq!(translations["Hello"], "Bonjour");
        assert_eq!(translations["file"], "fichier");
    }

    #[test]
    fn parses_big_endian() {
        let bytes = build(&[("Hello", "Hallo")], true);
        assert_eq!(parse(&bytes).unwrap()["Hello"], "Hallo");
    }

    #[test]
    fn rejects_invalid_magic() {
        assert_eq!(
            parse(&[0; 28]).err().unwrap(),
            "MO parse error: invalid magic number"
        );
    }

    #[test]
    fn rejects_truncated_file() {
        let bytes = build(&[("Hello", "Bonjour")], false);
        assert!(parse(&bytes[..bytes.len() - 10]).is_err());
        assert!(parse(&bytes[..10]).is_err());
    }
}
