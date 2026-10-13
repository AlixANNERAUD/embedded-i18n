# Translating

[`translate!`] replaces a message key with its translation, at compile time.
The expansion is a plain string literal, so the translation costs nothing at runtime and can be used anywhere a literal can:

```rust
use embedded_i18n::translate;

const GREETING: &str = translate!("greeting");
static FAREWELL: &str = translate!("farewell");

assert_eq!(GREETING, "Hello");
assert_eq!(FAREWELL, "Goodbye");
```

With `EMBEDDED_I18N_LOCALE=fr`, the same code embeds `"Bonjour"` and `"Au revoir"` instead, and the English strings are not in the binary.

## Keys

The key is a string literal.
Escapes are unescaped like in any Rust string, so `translate!("Say \"hi\"\n")` looks up the key `Say "hi"` followed by a newline.
Keys of nested locale files are written with dots (`translate!("menu.settings.title")`), see [Locale Files][guide-locale-files].

## C Strings

A C string literal key returns a `&'static CStr`, to pass translations to C libraries (LVGL, display drivers...) without copying:

```rust
use core::ffi::CStr;
use embedded_i18n::translate;

let greeting: &'static CStr = translate!(c"c_greeting");
assert_eq!(greeting.to_bytes_with_nul(), b"Hello from C\0");
```

A translation containing a NUL byte cannot be a C string and is a compile error.

## Lookup

The key is looked up in the selected locale, then in each locale of the fallback chain (`fr-CA`, `fr`, then `EMBEDDED_I18N_FALLBACK`, `en` by default), and the first non-empty translation wins.
See [Configuration][guide-configuration] for the chain and [Coverage][guide-coverage] to forbid falling back.

## Other Macros

| Macro | Guide |
| --- | --- |
| [`translate_args!`], [`translate_write!`] | [Formatting][guide-formatting] |
| [`translate_plural!`], [`translate_plural_write!`] | [Plural Forms][guide-plurals] |
| [`translate_select!`], [`translate_select_write!`] | [Select Variants][guide-select] |
| [`track_locales!`] | [Rebuild Tracking][guide-rebuild-tracking] |

Every macro accepts a [message context][guide-context] before the key.

## Compile Errors

Mistakes are reported at the call site, as regular compile errors:

```rust,compile_fail
let _ = embedded_i18n::translate!("no such key");
```

```text
error: translation for "no such key" not found in locales "en" (/path/to/crate/locales)
```

- a key missing from every locale of the chain;
- a key that is not a string or C string literal:

  ```rust,compile_fail
  let key = "greeting";
  let _ = embedded_i18n::translate!(key);
  ```

- a translation with a NUL byte requested as a C string;
- every error of the locale files themselves (see [Locale Files][guide-locale-files]).

[guide-locale-files]: locale_files.md
[guide-configuration]: configuration.md
[guide-coverage]: coverage.md
[guide-formatting]: formatting.md
[guide-plurals]: plurals.md
[guide-select]: select.md
[guide-rebuild-tracking]: rebuild_tracking.md
[guide-context]: context.md
[`translate!`]: translating.md
[`translate_args!`]: formatting.md
[`translate_write!`]: formatting.md
[`translate_plural!`]: plurals.md
[`translate_plural_write!`]: plurals.md
[`translate_select!`]: select.md
[`translate_select_write!`]: select.md
[`track_locales!`]: rebuild_tracking.md
