# Configuration

The macros are configured with environment variables, read at compile time.
Set them on the command line for a one-off build:

```sh
EMBEDDED_I18N_LOCALE=fr cargo build --release
```

or once in `.cargo/config.toml`, to share them with your team and CI:

```toml
[env]
EMBEDDED_I18N_LOCALE = "fr"
EMBEDDED_I18N_FALLBACK = "en"
```

Changing a variable always rebuilds the crates using the macros.

## Variables

| Variable | Default | Description |
| --- | --- | --- |
| `EMBEDDED_I18N_LOCALE` | `en` | Locale to embed, or the locale active at startup with `EMBEDDED_I18N_LOCALES` |
| `EMBEDDED_I18N_FALLBACK` | `en` | Comma separated fallback locales, tried in order |
| `EMBEDDED_I18N_PATH` | `locales` | Locale files directory, relative to the `Cargo.toml` of the crate calling the macros, or absolute |
| `EMBEDDED_I18N_LOCALES` | unset | Comma separated locales to embed for [runtime selection][guide-runtime-locales] |
| `EMBEDDED_I18N_STRICT` | unset | Forbids falling back for messages missing from the primary locale, see [Coverage][guide-coverage] |
| `EMBEDDED_I18N_BIDI_ISOLATION` | unset | Isolates the placeholders of right-to-left translations, see [Right-to-Left Languages][guide-right-to-left] |

The boolean variables are enabled by any value except an empty string, `0`, `false`, `no` and `off`, case-insensitively.

`EMBEDDED_I18N_LOCALE=pseudo` builds a [pseudo-localization][guide-pseudo-localization] of the fallback locales.

The values are also available to the code at compile time: [`get_locale`] and [`get_fallback_locale`] return `EMBEDDED_I18N_LOCALE` and `EMBEDDED_I18N_FALLBACK` as `&'static str`, in `const` context:

```rust
const LOCALE: &str = embedded_i18n::get_locale();
assert_eq!(LOCALE, "en");
```

## Locale Resolution

A message is looked up in a chain of locales, from the most specific to the most generic:

1. the selected locale (`EMBEDDED_I18N_LOCALE`);
2. the same locale without its last subtag, repeatedly: `zh-Hant-TW`, `zh-Hant`, `zh`;
3. each fallback locale (`EMBEDDED_I18N_FALLBACK`), in order, with the same subtag removal.

With `EMBEDDED_I18N_LOCALE=fr-CA` and `EMBEDDED_I18N_FALLBACK=de,en`, the chain is `fr-CA`, `fr`, `de`, `en`.
A locale appearing twice is only tried once.
The first non-empty translation wins, and a message missing from every locale is a compile error listing the chain.

Locale identifiers are normalized like BCP 47 tags: `fr_ca`, `FR-CA` and `fr-CA` are the same locale, scripts are title-cased (`zh-hant` becomes `zh-Hant`) and regions upper-cased.
The file of a locale can be named with `-` or `_`, in its normalized case or in lower case: `fr-CA.json`, `fr_CA.json`, `fr-ca.json` or `fr_ca.json`.
Missing files are skipped.

## Locale Files Directory

By default, locale files are read from `locales/`, next to the `Cargo.toml` of the crate calling the macros.
`EMBEDDED_I18N_PATH` changes it, relative to that `Cargo.toml` or absolute.

To share one set of locale files between the crates of a workspace, set it once in the `.cargo/config.toml` of the workspace root.
`relative = true` makes the path relative to the directory containing `.cargo/`:

```toml
[env]
EMBEDDED_I18N_PATH = { value = "i18n", relative = true }
```

A missing directory is a compile error naming the resolved path:

```text
error: locales directory not found: /path/to/crate/i18n
```

## Build Scripts

With the `std` feature, [`get_locale_build`] and [`get_fallback_locale_build`] read the same variables from the environment of a running build script, for instance to pick the font subset of the locale (see [Unicode Ranges][guide-unicode-ranges]).

[guide-runtime-locales]: runtime_locales.md
[guide-coverage]: coverage.md
[guide-right-to-left]: right_to_left.md
[guide-pseudo-localization]: pseudo_localization.md
[guide-unicode-ranges]: unicode_ranges.md
[`get_locale`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/fn.get_locale.html
[`get_fallback_locale`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/fn.get_fallback_locale.html
[`get_locale_build`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/fn.get_locale_build.html
[`get_fallback_locale_build`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/fn.get_fallback_locale_build.html
