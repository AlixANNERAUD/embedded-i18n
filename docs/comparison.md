# Comparison With Other Crates

`embedded-i18n` resolves translations at compile time and embeds only the strings of the languages you build.
That fits firmware, where flash, RAM and the allocator are scarce and the set of languages is known before flashing.
Runtime libraries fit applications that load language packs at runtime or need the full expressiveness of Fluent.

| | embedded-i18n | [rust-i18n](https://crates.io/crates/rust-i18n) | [Fluent](https://crates.io/crates/fluent) | [gettext-rs](https://crates.io/crates/gettext-rs) |
| --- | --- | --- | --- | --- |
| `no_std`, no allocator | ✅ | ❌ | ❌ | ❌ |
| Translations resolved | At compile time, as literals | Embedded at compile time, looked up at runtime | Parsed and resolved at runtime | Loaded from `.mo` files at runtime |
| Missing translation | Compile error | Runtime fallback | Runtime fallback or error | Runtime fallback to the source string |
| Placeholders checked at compile time | ✅ | ❌ | ❌ | ❌ |
| Plural forms | CLDR rules, gettext `Plural-Forms` | Not built in | CLDR rules | gettext `Plural-Forms` |
| Message context / select variants | ✅ / ✅ | ❌ / ❌ | Message ids / ✅ | ✅ / ❌ |
| Runtime locale switching | Optional, locales chosen at build time | ✅ | ✅ | ✅ |
| File formats | JSON, YAML, TOML, PO, MO, Fluent (subset) | YAML, JSON, TOML | Fluent | MO |
| Localized dates | Built in, without `libc` | ❌ | ❌ | ❌ |

The other crates evolve: check their documentation for the current state.

## Coming From rust-i18n

- `t!("key")` becomes [`translate!`]`("key")`, and `t!("key", name = value)` becomes [`translate_write!`] or [`translate_args!`] with the same named arguments, checked at compile time.
- The YAML, JSON and TOML files can be reused with the matching backend features, one file per locale (`en.yml`, `fr.yml`). Files containing several locales at once are not supported.
- `rust_i18n::i18n!("locales")` has no equivalent: the directory is set with `EMBEDDED_I18N_PATH`, and [`track_locales!`] makes Cargo rebuild when the files change.
- Placeholders use Rust format syntax, `{name}`, instead of `%{name}`.
- `rust_i18n::set_locale("fr")` becomes [`set_locale`]`("fr")`, with the locales listed at build time in `EMBEDDED_I18N_LOCALES`.

## Coming From gettext

- `.po` and `.mo` files are read directly by the `backend-po` and `backend-mo` features, with `msgctxt`, plural forms, `Plural-Forms` headers and fuzzy entries handled like `msgfmt`.
- `gettext("text")` becomes [`translate!`]`("text")`, `pgettext("ctx", "text")` becomes `translate!(context = "ctx", "text")`, and `ngettext("file", "files", n)` becomes [`translate_plural!`]`("file", n)`.
- Translations are format strings: placeholders use Rust format syntax (`{}`, `{name}`) rather than `printf` (`%s`, `%d`).
- Source strings must be translated in at least one locale file (an English catalog, or `EMBEDDED_I18N_FALLBACK` pointing to the source language), since keys are not used as their own translation.

## Coming From Fluent

- Simple messages, terms, attributes, variables and whole-message selects are supported (see [Locale Files][guide-locale-files]).
- Functions (`NUMBER()`, `DATETIME()`), parameterized terms and selects mixed with text are not, as they need a runtime resolver.
- `{ $user-name }` becomes the Rust placeholder `{user_name}`.

[guide-locale-files]: locale_files.md
[`translate!`]: translating.md
[`translate_write!`]: formatting.md
[`translate_args!`]: formatting.md
[`translate_plural!`]: plurals.md
[`track_locales!`]: rebuild_tracking.md
[`set_locale`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/fn.set_locale.html
