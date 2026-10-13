use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Lit, LitCStr, LitStr};

#[cfg(feature = "backend-json")]
mod json;
#[cfg(feature = "backend-po")]
mod po;

struct Catalog {
    path: PathBuf,
    locale: String,
    fallback: String,
    translations: HashMap<String, String>,
}

static CATALOG: LazyLock<Result<Catalog, String>> = LazyLock::new(|| {
    let manifest_directory = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR is not set".to_string())?;
    let locale = std::env::var("EMBEDDED_I18N_LOCALE").unwrap_or_else(|_| "en".to_string());
    let fallback = std::env::var("EMBEDDED_I18N_FALLBACK").unwrap_or_else(|_| "en".to_string());
    load_catalog(
        &PathBuf::from(manifest_directory).join("locales"),
        &locale,
        &fallback,
    )
});

fn load_catalog(path: &Path, locale: &str, fallback: &str) -> Result<Catalog, String> {
    if !path.is_dir() {
        return Err(format!("locales directory not found: {}", path.display()));
    }

    let mut translations = HashMap::new();

    if let Some(entries) = load_locale(path, locale)? {
        for (key, value) in entries {
            if !value.is_empty() {
                translations.insert(key, value);
            }
        }
    }

    if let Some(entries) = load_locale(path, fallback)? {
        for (key, value) in entries {
            translations.entry(key).or_insert(value);
        }
    }

    Ok(Catalog {
        path: path.to_path_buf(),
        locale: locale.to_string(),
        fallback: fallback.to_string(),
        translations,
    })
}

#[allow(unused_variables)]
fn load_locale(path: &Path, locale: &str) -> Result<Option<HashMap<String, String>>, String> {
    let locale = locale.to_lowercase();

    #[cfg(feature = "backend-json")]
    {
        let file = path.join(format!("{}.{}", locale, json::EXTENSION));
        if file.exists() {
            return json::load(&file)
                .map(Some)
                .map_err(|e| format!("{}: {}", file.display(), e));
        }
    }

    #[cfg(feature = "backend-po")]
    {
        let file = path.join(format!("{}.{}", locale, po::EXTENSION));
        if file.exists() {
            return po::load(&file)
                .map(Some)
                .map_err(|e| format!("{}: {}", file.display(), e));
        }
    }

    Ok(None)
}

enum Key {
    Str(LitStr),
    CStr(LitCStr),
}

impl Key {
    fn span(&self) -> Span {
        match self {
            Key::Str(literal) => literal.span(),
            Key::CStr(literal) => literal.span(),
        }
    }

    fn value(&self) -> syn::Result<String> {
        match self {
            Key::Str(literal) => Ok(literal.value()),
            Key::CStr(literal) => literal
                .value()
                .into_string()
                .map_err(|_| syn::Error::new(literal.span(), "C string key must be valid UTF-8")),
        }
    }
}

impl Parse for Key {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match input.parse::<Lit>()? {
            Lit::Str(literal) => Ok(Key::Str(literal)),
            Lit::CStr(literal) => Ok(Key::CStr(literal)),
            other => Err(syn::Error::new(
                other.span(),
                "expected a string literal or a C string literal",
            )),
        }
    }
}

struct Input {
    key: Key,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
        }
        if !input.is_empty() {
            return Err(input.error("unexpected argument"));
        }
        Ok(Input { key })
    }
}

fn expand(input: TokenStream2, catalog: &Result<Catalog, String>) -> syn::Result<TokenStream2> {
    let input: Input = syn::parse2(input)?;
    let span = input.key.span();
    let catalog = catalog
        .as_ref()
        .map_err(|error| syn::Error::new(span, error))?;
    let key = input.key.value()?;

    let value = catalog.translations.get(&key).ok_or_else(|| {
        syn::Error::new(
            span,
            format!(
                "translation for {:?} not found in locale {:?} or fallback {:?} ({})",
                key,
                catalog.locale,
                catalog.fallback,
                catalog.path.display()
            ),
        )
    })?;

    match input.key {
        Key::Str(_) => {
            let value = LitStr::new(value, span);
            Ok(quote! { #value })
        }
        Key::CStr(_) => {
            let value = CString::new(value.as_str()).map_err(|_| {
                syn::Error::new(
                    span,
                    format!("translation for {:?} contains a nul byte", key),
                )
            })?;
            let value = LitCStr::new(&value, span);
            Ok(quote! { #value })
        }
    }
}

#[proc_macro]
pub fn translate(input: TokenStream) -> TokenStream {
    expand(input.into(), &CATALOG)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(entries: &[(&str, &str)]) -> Result<Catalog, String> {
        Ok(Catalog {
            path: PathBuf::from("locales"),
            locale: "fr".to_string(),
            fallback: "en".to_string(),
            translations: entries
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        })
    }

    fn error(input: TokenStream2, catalog: &Result<Catalog, String>) -> String {
        match expand(input, catalog) {
            Ok(tokens) => panic!("expected an error, got {}", tokens),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn expands_string_literal() {
        let tokens = expand(quote! { "greeting" }, &catalog(&[("greeting", "Bonjour")])).unwrap();
        assert_eq!(tokens.to_string(), quote! { "Bonjour" }.to_string());
    }

    #[test]
    fn expands_c_string_literal() {
        let tokens = expand(quote! { c"greeting" }, &catalog(&[("greeting", "Bonjour")])).unwrap();
        assert_eq!(tokens.to_string(), "c\"Bonjour\"");
    }

    #[test]
    fn accepts_trailing_comma() {
        assert!(expand(quote! { "greeting", }, &catalog(&[("greeting", "Bonjour")])).is_ok());
    }

    #[test]
    fn unescapes_key() {
        let tokens = expand(
            quote! { "say \"hi\"\n" },
            &catalog(&[("say \"hi\"\n", "dis \"salut\"")]),
        )
        .unwrap();
        let value: LitStr = syn::parse2(tokens).unwrap();
        assert_eq!(value.value(), "dis \"salut\"");
    }

    #[test]
    fn escapes_value() {
        let tokens = expand(quote! { "quote" }, &catalog(&[("quote", "a \"b\" \\ c")])).unwrap();
        let value: LitStr = syn::parse2(tokens).unwrap();
        assert_eq!(value.value(), "a \"b\" \\ c");
    }

    #[test]
    fn rejects_identifier() {
        assert!(error(quote! { greeting }, &catalog(&[])).contains("expected"));
    }

    #[test]
    fn rejects_non_string_literal() {
        assert_eq!(
            error(quote! { 42 }, &catalog(&[])),
            "expected a string literal or a C string literal"
        );
    }

    #[test]
    fn rejects_extra_argument() {
        assert_eq!(
            error(quote! { "greeting" "other" }, &catalog(&[])),
            "unexpected argument"
        );
    }

    #[test]
    fn rejects_empty_input() {
        assert!(!error(quote! {}, &catalog(&[])).is_empty());
    }

    #[test]
    fn reports_missing_key() {
        assert_eq!(
            error(quote! { "missing" }, &catalog(&[])),
            "translation for \"missing\" not found in locale \"fr\" or fallback \"en\" (locales)"
        );
    }

    #[test]
    fn reports_catalog_error() {
        assert_eq!(
            error(quote! { "greeting" }, &Err("broken".to_string())),
            "broken"
        );
    }

    #[test]
    fn reports_nul_byte_in_c_string() {
        assert!(error(quote! { c"nul" }, &catalog(&[("nul", "a\0b")])).contains("nul byte"));
    }

    #[test]
    fn reports_missing_locales_directory() {
        let directory = tempfile::tempdir().unwrap();
        let error = load_catalog(&directory.path().join("missing"), "en", "en")
            .err()
            .unwrap();
        assert!(error.starts_with("locales directory not found"));
    }

    #[cfg(feature = "backend-json")]
    #[test]
    fn reports_invalid_locale_file() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("en.json"), "{").unwrap();
        let error = load_catalog(directory.path(), "en", "en").err().unwrap();
        assert!(error.contains("en.json"));
    }

    #[cfg(feature = "backend-json")]
    #[test]
    fn primary_locale_overrides_fallback() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("fr.json"),
            r#"{"greeting": "Bonjour", "empty": ""}"#,
        )
        .unwrap();
        std::fs::write(
            directory.path().join("en.json"),
            r#"{"greeting": "Hello", "empty": "Empty", "only": "Only"}"#,
        )
        .unwrap();
        let catalog = load_catalog(directory.path(), "fr", "en").unwrap();
        assert_eq!(catalog.translations["greeting"], "Bonjour");
        assert_eq!(catalog.translations["empty"], "Empty");
        assert_eq!(catalog.translations["only"], "Only");
    }
}
