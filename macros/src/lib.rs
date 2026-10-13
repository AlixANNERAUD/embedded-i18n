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

type Translations = HashMap<String, String>;

struct Catalog {
    path: PathBuf,
    locale: String,
    fallback: String,
    files: Vec<PathBuf>,
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
    let mut files = Vec::new();

    if let Some((file, entries)) = load_locale(path, locale)? {
        files.push(file);
        for (key, value) in entries {
            if !value.is_empty() {
                translations.insert(key, value);
            }
        }
    }

    if let Some((file, entries)) = load_locale(path, fallback)? {
        if !files.contains(&file) {
            files.push(file);
        }
        for (key, value) in entries {
            translations.entry(key).or_insert(value);
        }
    }

    Ok(Catalog {
        path: path.to_path_buf(),
        locale: locale.to_string(),
        fallback: fallback.to_string(),
        files,
        translations,
    })
}

#[allow(unused_variables)]
fn load_locale(path: &Path, locale: &str) -> Result<Option<(PathBuf, Translations)>, String> {
    let locale = locale.to_lowercase();

    #[cfg(feature = "backend-json")]
    {
        let file = path.join(format!("{}.{}", locale, json::EXTENSION));
        if file.exists() {
            return json::load(&file)
                .map(|entries| Some((file.clone(), entries)))
                .map_err(|e| format!("{}: {}", file.display(), e));
        }
    }

    #[cfg(feature = "backend-po")]
    {
        let file = path.join(format!("{}.{}", locale, po::EXTENSION));
        if file.exists() {
            return po::load(&file)
                .map(|entries| Some((file.clone(), entries)))
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

fn translation(key: &Key, catalog: &Result<Catalog, String>) -> syn::Result<TokenStream2> {
    let span = key.span();
    let catalog = catalog
        .as_ref()
        .map_err(|error| syn::Error::new(span, error))?;
    let name = key.value()?;

    let value = catalog.translations.get(&name).ok_or_else(|| {
        syn::Error::new(
            span,
            format!(
                "translation for {:?} not found in locale {:?} or fallback {:?} ({})",
                name,
                catalog.locale,
                catalog.fallback,
                catalog.path.display()
            ),
        )
    })?;

    match key {
        Key::Str(_) => {
            let value = LitStr::new(value, span);
            Ok(quote! { #value })
        }
        Key::CStr(_) => {
            let value = CString::new(value.as_str()).map_err(|_| {
                syn::Error::new(
                    span,
                    format!("translation for {:?} contains a nul byte", name),
                )
            })?;
            let value = LitCStr::new(&value, span);
            Ok(quote! { #value })
        }
    }
}

fn expand(input: TokenStream2, catalog: &Result<Catalog, String>) -> syn::Result<TokenStream2> {
    let input: Input = syn::parse2(input)?;
    translation(&input.key, catalog)
}

struct FormatInput {
    key: Key,
    arguments: TokenStream2,
}

impl Parse for FormatInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = match input.parse()? {
            Key::Str(literal) => Key::Str(literal),
            Key::CStr(literal) => {
                return Err(syn::Error::new(
                    literal.span(),
                    "expected a string literal, C strings cannot be used as format strings",
                ));
            }
        };
        if !input.is_empty() {
            input.parse::<syn::Token![,]>()?;
        }
        Ok(FormatInput {
            key,
            arguments: input.parse()?,
        })
    }
}

struct WriteInput {
    writer: syn::Expr,
    format: FormatInput,
}

impl Parse for WriteInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let writer = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        Ok(WriteInput {
            writer,
            format: input.parse()?,
        })
    }
}

fn expand_args(
    input: TokenStream2,
    catalog: &Result<Catalog, String>,
) -> syn::Result<TokenStream2> {
    let input: FormatInput = syn::parse2(input)?;
    let format = translation(&input.key, catalog)?;
    let arguments = input.arguments;
    Ok(quote! { ::core::format_args!(#format, #arguments) })
}

fn expand_write(
    input: TokenStream2,
    catalog: &Result<Catalog, String>,
) -> syn::Result<TokenStream2> {
    let input: WriteInput = syn::parse2(input)?;
    let writer = input.writer;
    let format = translation(&input.format.key, catalog)?;
    let arguments = input.format.arguments;
    Ok(quote! { ::core::write!(#writer, #format, #arguments) })
}

fn expand_track_locales(
    input: TokenStream2,
    catalog: &Result<Catalog, String>,
) -> syn::Result<TokenStream2> {
    if !input.is_empty() {
        return Err(syn::Error::new_spanned(input, "unexpected argument"));
    }
    let catalog = catalog
        .as_ref()
        .map_err(|error| syn::Error::new(Span::call_site(), error))?;
    let files = catalog
        .files
        .iter()
        .map(|file| file.to_string_lossy().into_owned());
    Ok(quote! {
        const _: () = {
            #(const _: &[u8] = include_bytes!(#files);)*
        };
    })
}

#[proc_macro]
pub fn track_locales(input: TokenStream) -> TokenStream {
    expand_track_locales(input.into(), &CATALOG)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn translate(input: TokenStream) -> TokenStream {
    expand(input.into(), &CATALOG)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn translate_args(input: TokenStream) -> TokenStream {
    expand_args(input.into(), &CATALOG)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn translate_write(input: TokenStream) -> TokenStream {
    expand_write(input.into(), &CATALOG)
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
            files: vec![
                PathBuf::from("/locales/fr.json"),
                PathBuf::from("/locales/en.json"),
            ],
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

    #[test]
    fn track_locales_includes_loaded_files() {
        let tokens = expand_track_locales(quote! {}, &catalog(&[])).unwrap();
        assert_eq!(
            tokens.to_string(),
            quote! {
                const _: () = {
                    const _: &[u8] = include_bytes!("/locales/fr.json");
                    const _: &[u8] = include_bytes!("/locales/en.json");
                };
            }
            .to_string()
        );
    }

    #[test]
    fn track_locales_rejects_arguments() {
        let error = expand_track_locales(quote! { "locales" }, &catalog(&[]))
            .err()
            .unwrap();
        assert_eq!(error.to_string(), "unexpected argument");
    }

    #[cfg(feature = "backend-json")]
    #[test]
    fn catalog_lists_loaded_files_once() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("fr.json"), "{}").unwrap();
        std::fs::write(directory.path().join("en.json"), "{}").unwrap();
        let catalog = load_catalog(directory.path(), "fr", "en").unwrap();
        assert_eq!(
            catalog.files,
            vec![
                directory.path().join("fr.json"),
                directory.path().join("en.json")
            ]
        );
        let catalog = load_catalog(directory.path(), "en", "en").unwrap();
        assert_eq!(catalog.files, vec![directory.path().join("en.json")]);
    }

    #[test]
    fn args_expands_to_format_args() {
        let tokens = expand_args(
            quote! { "greeting", name = user, 42 },
            &catalog(&[("greeting", "Bonjour {name} {}")]),
        )
        .unwrap();
        assert_eq!(
            tokens.to_string(),
            quote! { ::core::format_args!("Bonjour {name} {}", name = user, 42) }.to_string()
        );
    }

    #[test]
    fn args_without_arguments() {
        let tokens =
            expand_args(quote! { "greeting" }, &catalog(&[("greeting", "Bonjour")])).unwrap();
        assert_eq!(
            tokens.to_string(),
            quote! { ::core::format_args!("Bonjour",) }.to_string()
        );
    }

    #[test]
    fn args_rejects_c_string() {
        assert!(
            expand_args(quote! { c"greeting" }, &catalog(&[("greeting", "Bonjour")]))
                .err()
                .unwrap()
                .to_string()
                .contains("C strings cannot be used as format strings")
        );
    }

    #[test]
    fn args_requires_comma() {
        assert!(
            expand_args(
                quote! { "greeting" 42 },
                &catalog(&[("greeting", "Bonjour")])
            )
            .is_err()
        );
    }

    #[test]
    fn write_expands_to_write() {
        let tokens = expand_write(
            quote! { &mut output, "greeting", 42 },
            &catalog(&[("greeting", "Bonjour {}")]),
        )
        .unwrap();
        assert_eq!(
            tokens.to_string(),
            quote! { ::core::write!(&mut output, "Bonjour {}", 42) }.to_string()
        );
    }

    #[test]
    fn write_requires_key() {
        assert!(expand_write(quote! { output }, &catalog(&[])).is_err());
    }

    #[test]
    fn write_reports_missing_key() {
        assert!(
            expand_write(quote! { output, "missing" }, &catalog(&[]))
                .err()
                .unwrap()
                .to_string()
                .starts_with("translation for \"missing\" not found")
        );
    }
}
