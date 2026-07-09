#![no_std]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod range;
mod time;

pub use range::*;
pub use time::*;

pub use embedded_i18n_macros::translate;

const DEFAULT_LOCALE: &str = "en";
const DEFAULT_FALLBACK_LOCALE: &str = "en";

pub const fn get_locale() -> &'static str {
    match option_env!("EMBEDDED_I18N_LOCALE") {
        Some(locale) => locale,
        None => DEFAULT_LOCALE,
    }
}

pub const fn get_fallback_locale() -> &'static str {
    match option_env!("EMBEDDED_I18N_FALLBACK") {
        Some(locale) => locale,
        None => DEFAULT_FALLBACK_LOCALE,
    }
}

#[cfg(feature = "std")]
pub fn get_locale_build() -> std::string::String {
    use std::string::ToString;

    match std::env::var("EMBEDDED_I18N_LOCALE") {
        Ok(locale) => locale,
        Err(_) => DEFAULT_LOCALE.to_string(),
    }
}

#[cfg(feature = "std")]
pub fn get_fallback_locale_build() -> std::string::String {
    use std::string::ToString;

    match std::env::var("EMBEDDED_I18N_FALLBACK") {
        Ok(locale) => locale,
        Err(_) => DEFAULT_FALLBACK_LOCALE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_locale_is_en() {
        assert_eq!(get_locale(), "en");
    }

    #[test]
    fn default_fallback_is_en() {
        assert_eq!(get_fallback_locale(), "en");
    }

    #[test]
    fn get_locale_ranges_supported_locales() {
        for locale in &[
            "en", "fr", "de", "es", "it", "pt", "nl", "pl", "ru", "ja", "zh",
        ] {
            assert!(
                get_locale_ranges(locale).is_some(),
                "{} should have ranges",
                locale
            );
        }
    }

    #[test]
    fn get_locale_ranges_unsupported_locale() {
        assert!(get_locale_ranges("unsupported").is_none());
        assert!(get_locale_ranges("xx").is_none());
        assert!(get_locale_ranges("").is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn get_locale_build_default() {
        let locale = get_locale_build();
        assert_eq!(locale, "en");
    }

    #[cfg(feature = "std")]
    #[test]
    fn get_fallback_locale_build_default() {
        let fallback = get_fallback_locale_build();
        assert_eq!(fallback, "en");
    }

    #[cfg(feature = "std")]
    #[test]
    fn get_locale_build_with_env() {
        let previous = std::env::var("EMBEDDED_I18N_LOCALE");
        unsafe { std::env::set_var("EMBEDDED_I18N_LOCALE", "fr") };
        assert_eq!(get_locale_build(), "fr");
        match previous {
            Ok(val) => unsafe { std::env::set_var("EMBEDDED_I18N_LOCALE", val) },
            Err(_) => unsafe { std::env::remove_var("EMBEDDED_I18N_LOCALE") },
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn get_fallback_locale_build_with_env() {
        let previous = std::env::var("EMBEDDED_I18N_FALLBACK");
        unsafe { std::env::set_var("EMBEDDED_I18N_FALLBACK", "de") };
        assert_eq!(get_fallback_locale_build(), "de");
        match previous {
            Ok(val) => unsafe { std::env::set_var("EMBEDDED_I18N_FALLBACK", val) },
            Err(_) => unsafe { std::env::remove_var("EMBEDDED_I18N_FALLBACK") },
        }
    }

    #[test]
    fn translate_macro_is_reexported() {
        let _ = translate!("greeting");
    }
}
