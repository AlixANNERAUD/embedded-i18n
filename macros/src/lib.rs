use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Lit, LitCStr, LitStr};

#[cfg(feature = "backend-fluent")]
mod fluent;
#[cfg(feature = "backend-json")]
mod json;
mod locale;
#[cfg(feature = "backend-mo")]
mod mo;
#[cfg(feature = "backend-po")]
mod po;
#[cfg(feature = "backend-toml")]
mod toml;
#[cfg(any(
    feature = "backend-json",
    feature = "backend-yaml",
    feature = "backend-toml"
))]
mod tree;
#[cfg(feature = "backend-yaml")]
mod yaml;

type Translations = HashMap<String, String>;

type Loader = fn(&Path) -> Result<Translations, String>;

const BACKENDS: &[(&[&str], Loader)] = &[
    #[cfg(feature = "backend-json")]
    (json::EXTENSIONS, json::load),
    #[cfg(feature = "backend-yaml")]
    (yaml::EXTENSIONS, yaml::load),
    #[cfg(feature = "backend-toml")]
    (toml::EXTENSIONS, toml::load),
    #[cfg(feature = "backend-po")]
    (po::EXTENSIONS, po::load),
    #[cfg(feature = "backend-mo")]
    (mo::EXTENSIONS, mo::load),
    #[cfg(feature = "backend-fluent")]
    (fluent::EXTENSIONS, fluent::load),
];

struct Catalog {
    path: PathBuf,
    chain: Vec<String>,
    files: Vec<PathBuf>,
    translations: HashMap<String, String>,
}

static CATALOG: LazyLock<Result<Catalog, String>> = LazyLock::new(|| {
    let manifest_directory = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR is not set".to_string())?;
    let locale = std::env::var("EMBEDDED_I18N_LOCALE").unwrap_or_else(|_| "en".to_string());
    let fallback = std::env::var("EMBEDDED_I18N_FALLBACK").unwrap_or_else(|_| "en".to_string());
    let path = locales_directory(
        Path::new(&manifest_directory),
        std::env::var("EMBEDDED_I18N_PATH").ok().as_deref(),
    );
    load_catalog(&path, &locale, &fallback)
});

fn locales_directory(manifest_directory: &Path, configured: Option<&str>) -> PathBuf {
    match configured {
        Some(path) if !path.is_empty() => manifest_directory.join(path),
        _ => manifest_directory.join("locales"),
    }
}

fn load_catalog(path: &Path, locale: &str, fallback: &str) -> Result<Catalog, String> {
    if !path.is_dir() {
        return Err(format!("locales directory not found: {}", path.display()));
    }

    let chain = locale::chain(locale, fallback);
    let mut translations = HashMap::new();
    let mut empty = HashMap::new();
    let mut files = Vec::new();

    for locale in &chain {
        if let Some((file, entries)) = load_locale(path, locale)? {
            if !files.contains(&file) {
                files.push(file);
            }
            for (key, value) in entries {
                if value.is_empty() {
                    empty.entry(key).or_insert(value);
                } else {
                    translations.entry(key).or_insert(value);
                }
            }
        }
    }

    for (key, value) in empty {
        translations.entry(key).or_insert(value);
    }

    Ok(Catalog {
        path: path.to_path_buf(),
        chain,
        files,
        translations,
    })
}

fn load_locale(path: &Path, locale: &str) -> Result<Option<(PathBuf, Translations)>, String> {
    for stem in locale::file_stems(locale) {
        for (extensions, load) in BACKENDS {
            for extension in *extensions {
                let file = path.join(format!("{}.{}", stem, extension));
                if file.is_file() {
                    return load(&file)
                        .map(|entries| Some((file.clone(), entries)))
                        .map_err(|e| format!("{}: {}", file.display(), e));
                }
            }
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
                "translation for {:?} not found in locales {} ({})",
                name,
                catalog
                    .chain
                    .iter()
                    .map(|locale| format!("{:?}", locale))
                    .collect::<Vec<_>>()
                    .join(", "),
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
            chain: vec!["fr".to_string(), "en".to_string()],
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
            "translation for \"missing\" not found in locales \"fr\", \"en\" (locales)"
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

    #[test]
    fn locales_directory_defaults_to_locales() {
        assert_eq!(
            locales_directory(Path::new("/crate"), None),
            PathBuf::from("/crate/locales")
        );
        assert_eq!(
            locales_directory(Path::new("/crate"), Some("")),
            PathBuf::from("/crate/locales")
        );
    }

    #[test]
    fn locales_directory_relative_to_manifest() {
        assert_eq!(
            locales_directory(Path::new("/workspace/app"), Some("../i18n")),
            PathBuf::from("/workspace/app/../i18n")
        );
    }

    #[test]
    fn locales_directory_absolute() {
        assert_eq!(
            locales_directory(Path::new("/workspace/app"), Some("/shared/i18n")),
            PathBuf::from("/shared/i18n")
        );
    }

    #[cfg(feature = "backend-json")]
    #[test]
    fn region_falls_back_to_language_then_fallbacks() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("fr_CA.json"),
            r#"{"color": "Couleur (CA)"}"#,
        )
        .unwrap();
        std::fs::write(
            directory.path().join("fr.json"),
            r#"{"color": "Couleur", "door": "Porte", "empty": ""}"#,
        )
        .unwrap();
        std::fs::write(directory.path().join("de.json"), r#"{"bus": "Bus (de)"}"#).unwrap();
        std::fs::write(
            directory.path().join("en.json"),
            r#"{"color": "Color", "door": "Door", "bus": "Bus", "train": "Train", "empty": ""}"#,
        )
        .unwrap();
        let catalog = load_catalog(directory.path(), "fr-ca", "de,en").unwrap();
        assert_eq!(catalog.chain, ["fr-CA", "fr", "de", "en"]);
        assert_eq!(catalog.translations["color"], "Couleur (CA)");
        assert_eq!(catalog.translations["door"], "Porte");
        assert_eq!(catalog.translations["bus"], "Bus (de)");
        assert_eq!(catalog.translations["train"], "Train");
        assert_eq!(catalog.translations["empty"], "");
        assert_eq!(catalog.files.len(), 4);
    }

    #[cfg(feature = "backend-json")]
    #[test]
    fn locale_file_names_are_matched_case_insensitively() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("zh-cn.json"), r#"{"hello": "你好"}"#).unwrap();
        let catalog = load_catalog(directory.path(), "zh_CN", "en").unwrap();
        assert_eq!(catalog.translations["hello"], "你好");
    }

    #[cfg(feature = "backend-json")]
    #[test]
    fn nested_json_keys_are_flattened() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("en.json"),
            r#"{"menu": {"settings": {"title": "Settings"}}}"#,
        )
        .unwrap();
        let catalog = load_catalog(directory.path(), "en", "en").unwrap();
        assert_eq!(catalog.translations["menu.settings.title"], "Settings");
    }

    #[cfg(feature = "backend-yaml")]
    #[test]
    fn loads_yaml_and_yml_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("fr.yml"), "menu:\n  open: Ouvrir\n").unwrap();
        std::fs::write(directory.path().join("en.yaml"), "menu:\n  open: Open\n").unwrap();
        let catalog = load_catalog(directory.path(), "fr", "en").unwrap();
        assert_eq!(catalog.translations["menu.open"], "Ouvrir");
        assert_eq!(catalog.files.len(), 2);
    }

    #[cfg(feature = "backend-toml")]
    #[test]
    fn loads_toml_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("en.toml"),
            "[menu]\nopen = \"Open\"\n",
        )
        .unwrap();
        let catalog = load_catalog(directory.path(), "en", "en").unwrap();
        assert_eq!(catalog.translations["menu.open"], "Open");
    }

    #[cfg(feature = "backend-fluent")]
    #[test]
    fn loads_fluent_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("en.ftl"),
            "hello = Hello, { $name }!\n",
        )
        .unwrap();
        let catalog = load_catalog(directory.path(), "en", "en").unwrap();
        assert_eq!(catalog.translations["hello"], "Hello, {name}!");
    }

    #[cfg(feature = "backend-mo")]
    #[test]
    fn reports_invalid_mo_file() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("en.mo"), [0u8; 28]).unwrap();
        let error = load_catalog(directory.path(), "en", "en").err().unwrap();
        assert!(error.contains("en.mo"));
    }

    #[cfg(all(feature = "backend-json", feature = "backend-po"))]
    #[test]
    fn json_takes_precedence_over_po() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("en.json"), r#"{"a": "json"}"#).unwrap();
        std::fs::write(
            directory.path().join("en.po"),
            "msgid \"a\"\nmsgstr \"po\"\n",
        )
        .unwrap();
        let catalog = load_catalog(directory.path(), "en", "en").unwrap();
        assert_eq!(catalog.translations["a"], "json");
    }
}
