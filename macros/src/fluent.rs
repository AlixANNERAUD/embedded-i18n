use std::{collections::HashMap, path::Path};

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};
use fluent_syntax::unicode::unescape_unicode_to_string;

pub const EXTENSIONS: &[&str] = &["ftl"];

const MAXIMUM_DEPTH: usize = 16;

pub fn load(path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    parse(&content)
}

type Attributes<'a> = HashMap<&'a str, &'a Pattern<&'a str>>;

struct Resource<'a> {
    messages: HashMap<&'a str, (Option<&'a Pattern<&'a str>>, Attributes<'a>)>,
    terms: HashMap<&'a str, (&'a Pattern<&'a str>, Attributes<'a>)>,
}

fn parse(content: &str) -> Result<HashMap<String, String>, String> {
    let resource = fluent_syntax::parser::parse(content).map_err(|(_, errors)| {
        format!(
            "Fluent parse error: {}",
            errors
                .iter()
                .map(|error| format!("{:?}", error.kind))
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let mut index = Resource {
        messages: HashMap::new(),
        terms: HashMap::new(),
    };

    for entry in &resource.body {
        match entry {
            Entry::Message(message) => {
                index.messages.insert(
                    message.id.name,
                    (
                        message.value.as_ref(),
                        message
                            .attributes
                            .iter()
                            .map(|attribute| (attribute.id.name, &attribute.value))
                            .collect(),
                    ),
                );
            }
            Entry::Term(term) => {
                index.terms.insert(
                    term.id.name,
                    (
                        &term.value,
                        term.attributes
                            .iter()
                            .map(|attribute| (attribute.id.name, &attribute.value))
                            .collect(),
                    ),
                );
            }
            _ => {}
        }
    }

    let mut translations = HashMap::new();

    for (id, (value, attributes)) in &index.messages {
        if let Some(value) = value {
            translations.insert(id.to_string(), resolve(&index, id, value, 0)?);
        }
        for (attribute, value) in attributes {
            let key = format!("{}.{}", id, attribute);
            let value = resolve(&index, &key, value, 0)?;
            translations.insert(key, value);
        }
    }

    Ok(translations)
}

fn resolve(
    resource: &Resource,
    key: &str,
    pattern: &Pattern<&str>,
    depth: usize,
) -> Result<String, String> {
    if depth > MAXIMUM_DEPTH {
        return Err(format!("Fluent error: reference cycle in {:?}", key));
    }

    let mut output = String::new();

    for element in &pattern.elements {
        match element {
            PatternElement::TextElement { value } => output.push_str(value),
            PatternElement::Placeable { expression } => {
                output.push_str(&expression_value(resource, key, expression, depth)?)
            }
        }
    }

    Ok(output)
}

fn expression_value(
    resource: &Resource,
    key: &str,
    expression: &Expression<&str>,
    depth: usize,
) -> Result<String, String> {
    let unsupported = |what: &str| {
        Err(format!(
            "Fluent error: {} in {:?} is not supported",
            what, key
        ))
    };

    match expression {
        Expression::Select { .. } => unsupported("select expression"),
        Expression::Inline(inline) => match inline {
            InlineExpression::StringLiteral { value } => {
                Ok(unescape_unicode_to_string(value).into_owned())
            }
            InlineExpression::NumberLiteral { value } => Ok(value.to_string()),
            InlineExpression::VariableReference { id } => {
                Ok(format!("{{{}}}", id.name.replace('-', "_")))
            }
            InlineExpression::MessageReference { id, attribute } => {
                let (value, attributes) = resource.messages.get(id.name).ok_or_else(|| {
                    format!("Fluent error: unknown message {:?} in {:?}", id.name, key)
                })?;
                let pattern = match attribute {
                    Some(attribute) => attributes.get(attribute.name).copied(),
                    None => *value,
                }
                .ok_or_else(|| format!("Fluent error: unresolved reference in {:?}", key))?;
                resolve(resource, key, pattern, depth + 1)
            }
            InlineExpression::TermReference {
                id,
                attribute,
                arguments,
            } => {
                if arguments.is_some() {
                    return unsupported("parameterized term");
                }
                let (value, attributes) = resource.terms.get(id.name).ok_or_else(|| {
                    format!("Fluent error: unknown term {:?} in {:?}", id.name, key)
                })?;
                let pattern = match attribute {
                    Some(attribute) => *attributes.get(attribute.name).ok_or_else(|| {
                        format!("Fluent error: unresolved reference in {:?}", key)
                    })?,
                    None => *value,
                };
                resolve(resource, key, pattern, depth + 1)
            }
            InlineExpression::FunctionReference { .. } => unsupported("function call"),
            InlineExpression::Placeable { expression } => {
                expression_value(resource, key, expression, depth)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_messages_and_attributes() {
        let translations = parse(
            "# Comment\n-brand = Memo\nhello = Hello, { $user-name }!\nabout = About { -brand }\nlogin = Log in\n    .placeholder = Email\nmultiline =\n    First line\n    second line\nliteral = { \"{\" } braces { 42 }\nref = { login.placeholder } / { hello }\n",
        )
        .unwrap();
        assert_eq!(translations["hello"], "Hello, {user_name}!");
        assert_eq!(translations["about"], "About Memo");
        assert_eq!(translations["login"], "Log in");
        assert_eq!(translations["login.placeholder"], "Email");
        assert_eq!(translations["multiline"], "First line\nsecond line");
        assert_eq!(translations["literal"], "{ braces 42");
        assert_eq!(translations["ref"], "Email / Hello, {user_name}!");
        assert!(!translations.contains_key("-brand"));
    }

    #[test]
    fn rejects_select_expressions() {
        let error = parse("emails = { $count ->\n    [one] One email\n   *[other] Emails\n}\n")
            .err()
            .unwrap();
        assert_eq!(
            error,
            "Fluent error: select expression in \"emails\" is not supported"
        );
    }

    #[test]
    fn rejects_functions() {
        assert!(parse("n = { NUMBER($n) }\n").is_err());
    }

    #[test]
    fn rejects_cycles() {
        assert!(
            parse("a = { b }\nb = { a }\n")
                .err()
                .unwrap()
                .contains("cycle")
        );
    }

    #[test]
    fn reports_syntax_errors() {
        assert!(
            parse("hello = { \n")
                .err()
                .unwrap()
                .starts_with("Fluent parse error")
        );
    }
}
