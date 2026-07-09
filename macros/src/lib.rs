use std::collections::HashMap;

use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use quote::quote;

#[cfg(feature = "backend-json")]
mod json;
#[cfg(feature = "backend-po")]
mod po;

static TRANSLATION_PATH: Lazy<std::path::PathBuf> = Lazy::new(|| {
    let path = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .expect("CARGO_MANIFEST_DIR is not set");
    path.join("locales")
        .canonicalize()
        .expect("Failed to canonicalize path")
});

static LOCALE: Lazy<String> =
    Lazy::new(|| std::env::var("EMBEDDED_I18N_LOCALE").unwrap_or_else(|_| "en".to_string()));

static FALLBACK_LOCALE: Lazy<String> =
    Lazy::new(|| std::env::var("EMBEDDED_I18N_FALLBACK").unwrap_or_else(|_| "en".to_string()));

static TRANSLATION_MAP: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut generated_items = HashMap::new();

    if let Some(translations) = load_locale(&LOCALE) {
        for (key, value) in translations {
            if !value.is_empty() {
                generated_items.insert(key, value);
            }
        }
    }

    if let Some(translations) = load_locale(&FALLBACK_LOCALE) {
        for (key, value) in translations {
            generated_items.entry(key).or_insert(value);
        }
    }

    generated_items
});

fn load_locale(locale: &str) -> Option<HashMap<String, String>> {
    let locale = &locale.to_lowercase();

    #[cfg(feature = "backend-json")]
    {
        let path = TRANSLATION_PATH.join(format!("{}.{}", locale, json::EXTENSION));
        if path.exists() {
            match json::load(&path) {
                Ok(map) => return Some(map),
                Err(e) => eprintln!("embedded-i18n: JSON error for {:?}: {}", path, e),
            }
        }
    }

    #[cfg(feature = "backend-po")]
    {
        let path = TRANSLATION_PATH.join(format!("{}.{}", locale, po::EXTENSION));
        if path.exists() {
            match po::load(&path) {
                Ok(map) => return Some(map),
                Err(e) => eprintln!("embedded-i18n: PO error for {:?}: {}", path, e),
            }
        }
    }

    None
}

#[proc_macro]
pub fn translate(input: TokenStream) -> TokenStream {
    let input = input.to_string();

    let identifier = input.trim();
    let (c, identifier) = if let Some(s) = identifier.strip_prefix("c\"") {
        (true, s)
    } else {
        (false, identifier.strip_prefix("\"").unwrap_or(identifier))
    };

    let identifier = identifier.strip_suffix("\"").unwrap_or(identifier);

    let value = TRANSLATION_MAP.get(identifier).cloned().unwrap_or_else(|| {
        panic!(
            "Translation for '{}' not found in locale or fallback (path: {:?})",
            identifier,
            TRANSLATION_PATH.clone()
        )
    });

    let value = if c {
        let c_string_value = syn::LitCStr::new(
            std::ffi::CString::new(value)
                .expect("Failed to create CString")
                .as_c_str(),
            proc_macro2::Span::call_site(),
        );
        quote! { #c_string_value }
    } else {
        let value = syn::LitStr::new(&value, proc_macro2::Span::call_site());
        quote! { #value }
    };

    value.into()
}
