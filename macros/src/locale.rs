pub fn normalize(locale: &str) -> String {
    let mut result = String::new();

    for (index, part) in locale
        .trim()
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .enumerate()
    {
        if index > 0 {
            result.push('-');
        }
        let alphabetic = part.chars().all(|c| c.is_ascii_alphabetic());
        let numeric = part.chars().all(|c| c.is_ascii_digit());
        if index == 0 {
            result.push_str(&part.to_ascii_lowercase());
        } else if part.len() == 4 && alphabetic {
            result.push_str(&part[..1].to_ascii_uppercase());
            result.push_str(&part[1..].to_ascii_lowercase());
        } else if (part.len() == 2 && alphabetic) || (part.len() == 3 && numeric) {
            result.push_str(&part.to_ascii_uppercase());
        } else {
            result.push_str(&part.to_ascii_lowercase());
        }
    }

    result
}

pub fn chain(locale: &str, fallbacks: &str) -> Vec<String> {
    let mut chain: Vec<String> = Vec::new();

    for locale in std::iter::once(locale).chain(fallbacks.split(',')) {
        let normalized = normalize(locale);
        let mut current = normalized.as_str();
        while !current.is_empty() {
            if !chain.iter().any(|existing| existing == current) {
                chain.push(current.to_string());
            }
            current = match current.rfind('-') {
                Some(index) => &current[..index],
                None => "",
            };
        }
    }

    chain
}

pub fn file_stems(locale: &str) -> Vec<String> {
    let lowercase = locale.to_ascii_lowercase();
    let mut stems: Vec<String> = Vec::new();

    for stem in [
        locale.to_string(),
        locale.replace('-', "_"),
        lowercase.clone(),
        lowercase.replace('-', "_"),
    ] {
        if !stems.contains(&stem) {
            stems.push(stem);
        }
    }

    stems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_separators() {
        assert_eq!(normalize("EN"), "en");
        assert_eq!(normalize("fr_ca"), "fr-CA");
        assert_eq!(normalize("fr-CA"), "fr-CA");
        assert_eq!(normalize("zh_hant_tw"), "zh-Hant-TW");
        assert_eq!(normalize("es-419"), "es-419");
        assert_eq!(normalize("de-CH-1996"), "de-CH-1996");
        assert_eq!(normalize(" pt-br "), "pt-BR");
        assert_eq!(normalize(""), "");
    }

    #[test]
    fn chain_truncates_subtags() {
        assert_eq!(
            chain("zh-Hant-TW", "en"),
            ["zh-Hant-TW", "zh-Hant", "zh", "en"]
        );
    }

    #[test]
    fn chain_accepts_fallback_list() {
        assert_eq!(
            chain("fr_CA", "fr-BE, de ,en"),
            ["fr-CA", "fr", "fr-BE", "de", "en"]
        );
    }

    #[test]
    fn chain_removes_duplicates_and_empty_entries() {
        assert_eq!(chain("en", "en,,EN"), ["en"]);
        assert_eq!(chain("", "fr"), ["fr"]);
    }

    #[test]
    fn file_stems_try_canonical_then_variants() {
        assert_eq!(file_stems("fr-CA"), ["fr-CA", "fr_CA", "fr-ca", "fr_ca"]);
        assert_eq!(file_stems("en"), ["en"]);
    }
}
