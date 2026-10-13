const FIRST_STRONG_ISOLATE: char = '\u{2068}';
const POP_DIRECTIONAL_ISOLATE: char = '\u{2069}';

const RIGHT_TO_LEFT: &[&str] = &[
    "ar", "arc", "ckb", "dv", "fa", "he", "iw", "ks", "ku-Arab", "ps", "sd", "syr", "ug", "ur",
    "yi",
];

pub fn is_right_to_left(locale: &str) -> bool {
    let mut parts = locale.split('-');
    let language = parts.next().unwrap_or_default();
    if parts.any(|part| part == "Arab" || part == "Hebr") {
        return true;
    }
    RIGHT_TO_LEFT.iter().any(|candidate| {
        *candidate == language || (candidate.contains('-') && locale.starts_with(candidate))
    })
}

enum Segment<'a> {
    Text(&'a str),
    Placeholder(&'a str),
}

fn segments(value: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let bytes = value.as_bytes();
    while index < bytes.len() {
        match bytes[index] {
            b'{' | b'}' if bytes.get(index + 1) == Some(&bytes[index]) => index += 2,
            b'{' => match value[index..].find('}') {
                Some(length) => {
                    if start < index {
                        segments.push(Segment::Text(&value[start..index]));
                    }
                    let end = index + length + 1;
                    segments.push(Segment::Placeholder(&value[index..end]));
                    start = end;
                    index = end;
                }
                None => index += 1,
            },
            _ => index += 1,
        }
    }
    if start < value.len() {
        segments.push(Segment::Text(&value[start..]));
    }
    segments
}

pub fn isolate_placeholders(value: &str) -> String {
    segments(value)
        .into_iter()
        .map(|segment| match segment {
            Segment::Text(text) => text.to_string(),
            Segment::Placeholder(placeholder) => {
                format!(
                    "{}{}{}",
                    FIRST_STRONG_ISOLATE, placeholder, POP_DIRECTIONAL_ISOLATE
                )
            }
        })
        .collect()
}

fn accent(character: char) -> char {
    match character {
        'a' => 'á',
        'c' => 'ç',
        'e' => 'é',
        'i' => 'í',
        'n' => 'ñ',
        'o' => 'ó',
        'u' => 'ú',
        'y' => 'ý',
        'A' => 'Á',
        'C' => 'Ç',
        'E' => 'É',
        'I' => 'Í',
        'N' => 'Ñ',
        'O' => 'Ó',
        'U' => 'Ú',
        'Y' => 'Ý',
        other => other,
    }
}

pub fn pseudo(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let mut output = String::from("[");
    let mut letters: usize = 0;
    for segment in segments(value) {
        match segment {
            Segment::Text(text) => {
                for character in text.chars() {
                    if character.is_alphanumeric() {
                        letters += 1;
                    }
                    output.push(accent(character));
                }
            }
            Segment::Placeholder(placeholder) => output.push_str(placeholder),
        }
    }
    let padding = letters.div_ceil(3);
    if padding > 0 {
        output.push(' ');
        output.extend(std::iter::repeat_n('~', padding));
    }
    output.push(']');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_right_to_left_locales() {
        for locale in [
            "ar", "ar-EG", "he", "fa-IR", "ur", "ku-Arab", "pa-Arab", "yi",
        ] {
            assert!(is_right_to_left(locale), "{}", locale);
        }
        for locale in ["en", "fr-CA", "ku", "arn", "zh-Hant-TW", "pa"] {
            assert!(!is_right_to_left(locale), "{}", locale);
        }
    }

    #[test]
    fn isolates_placeholders_only() {
        assert_eq!(
            isolate_placeholders("مرحبا {name}، لديك {} رسائل {{literal}} {count:>3}"),
            "مرحبا \u{2068}{name}\u{2069}، لديك \u{2068}{}\u{2069} رسائل {{literal}} \u{2068}{count:>3}\u{2069}"
        );
        assert_eq!(isolate_placeholders("no placeholder {"), "no placeholder {");
        assert_eq!(isolate_placeholders(""), "");
    }

    #[test]
    fn pseudo_accents_expands_and_brackets() {
        assert_eq!(pseudo("Open"), "[Ópéñ ~~]");
        assert_eq!(pseudo("Hello {name}, {{x}}!"), "[Hélló {name}, {{x}}! ~~]");
        assert_eq!(pseudo("{count}"), "[{count}]");
        assert_eq!(pseudo(""), "");
    }
}
