# embedded-i18n

[![crates.io](https://img.shields.io/crates/v/embedded-i18n.svg)](https://crates.io/crates/embedded-i18n)
[![docs.rs](https://img.shields.io/docsrs/embedded-i18n)](https://docs.rs/embedded-i18n)
[![codecov](https://codecov.io/github/AlixANNERAUD/embedded-i18n/graph/badge.svg?token=0C27GIS0BK)](https://codecov.io/github/AlixANNERAUD/embedded-i18n)

Compile-time internationalization (i18n) for embedded and `no_std` systems. Translates strings at build time from JSON, YAML, TOML, GNU gettext PO/MO or Fluent locale files and embeds them directly into ROM, no runtime loading, no allocator required for translations.

## Installation

```toml
[dependencies]
embedded-i18n = "0.1"
```

## Quick Start

1. Place `locales/<locale>.json` or `locales/<locale>.po` files next to your `Cargo.toml`.
2. Set `EMBEDDED_I18N_LOCALE` (and optionally `EMBEDDED_I18N_FALLBACK`) at build time.
3. Call `track_locales!()` once at the root of your crate so that editing a locale file triggers a rebuild.
4. Use the `translate!()` macro, it resolves to a `&'static str` at compile time.

```rust
use embedded_i18n::translate;

embedded_i18n::track_locales!();

let greeting = translate!("Hello, world!");
```

### Locale files - JSON

```json
{
  "Hello, world!": "Bonjour le monde !",
  "Missing argument: {}": "Argument manquant : {}",
  "menu": {
    "settings": {
      "title": "Paramètres"
    }
  }
}
```

Nested objects are flattened to dotted keys: `translate!("menu.settings.title")`. Values must be strings or objects, and a key defined twice (e.g. `"menu.settings.title"` and the nested form) is a compile error.

### Locale files - YAML and TOML

```yaml
greeting: Bonjour
menu:
  settings:
    title: Paramètres
```

```toml
greeting = "Bonjour"

[menu.settings]
title = "Paramètres"
```

Nesting works like JSON. YAML files can use the `.yaml` or `.yml` extension.

### Locale files - GNU gettext PO

```po
msgid "Hello, world!"
msgstr "Bonjour le monde !"

msgid "Missing argument: {}"
msgstr "Argument manquant : {}"
```

Like `msgfmt`, entries flagged `#, fuzzy` and entries with an empty `msgstr` are ignored, so they resolve through the fallback locale.

Compiled gettext catalogs (`.mo`, little or big endian) are supported by the `backend-mo` feature.

### Locale files - Fluent

```ftl
-brand = Acme
hello = Hello, { $user-name }!
about = About { -brand }
login = Log in
    .placeholder = Email
```

Simple Fluent messages are supported: text, message and term references, string and number literals, and attributes (as `login.placeholder`). Variables become named format placeholders (`{ $user-name }` → `{user_name}`) for `translate_args!()` and `translate_write!()`. Select expressions, functions and parameterized terms are reported as compile errors.

### Build configuration

```sh
EMBEDDED_I18N_LOCALE=fr EMBEDDED_I18N_FALLBACK=en cargo build
```

## API Reference

### `translate!()`

The core macro. Replaces a string literal with its translation at compile time.

```rust
// Basic string
let s: &'static str = translate!("Hello");

// C string (for FFI)
let c: &'static CStr = translate!(c"Hello");
```

Fails to compile if the key is not found in either the selected locale or the fallback locale. The expansion is a plain literal, so it can be used in `const` items and as the format string of `format!` / `write!`.

### Message context

The same source string can need different translations (`Open` on a menu vs `Open` as a state). Pass a `context` before the key:

```rust
let action = translate!(context = "menu", "Open");
let state = translate!(context = "state", "Open");
```

`context` is accepted by every macro (`translate!`, `translate_args!`, `translate_write!`) and works with C strings. The context is looked up as:

- GNU gettext PO / MO: the `msgctxt` of the entry.

  ```po
  msgctxt "menu"
  msgid "Open"
  msgstr "Ouvrir"
  ```

- JSON, YAML, TOML and Fluent: a namespace named after the context, i.e. the key `menu.Open`.

  ```json
  { "menu": { "Open": "Ouvrir" } }
  ```

A key with a context never falls back to the same key without context.

### `translate_args!()` and `translate_write!()`

Format a translation with arguments without allocating. The translated string becomes the format string, so placeholders are checked against the arguments at compile time:

```rust
use core::fmt::Write;
use embedded_i18n::{translate_args, translate_write};

// "Hello {name}, you have {count} messages"
let name = "Ada";
translate_write!(&mut display, "inbox", count = 3)?;

// Anywhere `format_args!` is accepted
log::info!("{}", translate_args!("inbox", name = name, count = 3));
```

- `translate_args!(key, args...)` expands to `core::format_args!(<translation>, args...)`.
- `translate_write!(writer, key, args...)` expands to `core::write!(writer, <translation>, args...)`, so the writer needs `core::fmt::Write` (or `std::io::Write`) in scope.
- Positional (`{}`), named (`{name}` with `name = value`) and implicitly captured variables (`{name}` with a `name` binding in scope) are supported.
- With an allocator, `format!(translate!(key), args...)` works too, but only with explicit arguments.

### `track_locales!()`

Cargo does not know that `translate!()` reads the locale files, so editing them would not rebuild your crate. Invoke `track_locales!()` once, at module level, to make the crate depend on every locale file loaded for the build:

```rust
embedded_i18n::track_locales!();
```

It expands to `include_bytes!` of each file inside an unused constant, so it adds nothing to the binary. Changing `EMBEDDED_I18N_LOCALE` or `EMBEDDED_I18N_FALLBACK` always triggers a rebuild.

### Locale resolution

Translations are loaded from files in the `locales/` directory (or `EMBEDDED_I18N_PATH`) at compile time. The enabled backends determine which file formats are tried, in this order: `.json`, `.yaml` / `.yml`, `.toml`, `.po`, `.mo`, `.ftl`. The locale and fallback are selected via environment variables:

| Env variable             | Default | Description                                                    |
| ------------------------ | ------- | -------------------------------------------------------------- |
| `EMBEDDED_I18N_LOCALE`   | `en`    | Primary locale                                                 |
| `EMBEDDED_I18N_FALLBACK` | `en`    | Comma separated fallback locales (keys missing from the primary locale are looked up here, in order) |
| `EMBEDDED_I18N_PATH`     | `locales` | Locale files directory, relative to the `Cargo.toml` of the crate calling `translate!()`, or absolute |

Locale identifiers are normalized like BCP 47 tags (`fr_ca`, `FR-CA` and `fr-CA` are the same locale), and region or script subtags are dropped one at a time before moving to the next fallback. With `EMBEDDED_I18N_LOCALE=fr-CA` and `EMBEDDED_I18N_FALLBACK=de,en`, a key is looked up in `fr-CA`, `fr`, `de`, then `en`. Locale files can be named `fr-CA`, `fr_CA`, `fr-ca` or `fr_ca`. Missing files are skipped and the first non-empty translation wins.

To share one set of locale files between the crates of a workspace, set the path once in `.cargo/config.toml` at the workspace root. `relative = true` makes it relative to that file:

```toml
[env]
EMBEDDED_I18N_PATH = { value = "i18n", relative = true }
```

A missing directory is reported as a compile error naming the resolved path.

### Unicode range helpers

Map locales to Unicode code-point ranges for font subsetting:

```rust
use embedded_i18n::{get_locale_ranges, merge_contiguous_ranges_in_place, write_ranges};

// Get ranges for a locale
let ranges = get_locale_ranges("ja").unwrap();
// -> [Basic Latin, Hiragana, Katakana, CJK Unified Ideographs]

// Sort and merge overlapping/adjacent ranges, without allocation
let mut buffer = [0..0, 0..0, 0..0, 0..0];
buffer.clone_from_slice(ranges);
let length = merge_contiguous_ranges_in_place(&mut buffer);

// Format for fontconfig-like syntax into any `core::fmt::Write`
write_ranges(&mut output, &buffer[..length])?;
// -> "32-126,12353-12438,12449-12538,19968-40958"
```

With the `alloc` feature, `merge_contiguous_ranges(Vec<Range<u32>>) -> Vec<Range<u32>>`, `format_range` and `format_ranges` return owned values instead.

Supported locales: `en`, `fr`, `de`, `es`, `it`, `pt`, `nl`, `sv`, `no`, `da`, `pl`, `cs`, `sk`, `hu`, `ro`, `tr`, `ru`, `uk`, `be`, `el`, `ar`, `he`, `ja`, `zh`, `zh-CN`, `zh-TW`, `ko`, `hi`, `th`, `vi`.

### Timestamp formatting

`strftime`-style formatting without libc, written into any `core::fmt::Write`:

```rust
use embedded_i18n::write_unix_timestamp;

write_unix_timestamp(&mut output, 0, "%Y-%m-%d %H:%M:%S")?;
// -> "1970-01-01 00:00:00"
```

With the `alloc` feature, `format_unix_timestamp(0, "%Y-%m-%d %H:%M:%S")` returns a `String`.

| Token | Description                           |
| ----- | ------------------------------------- |
| `%Y`  | Year with century (e.g. `2026`)       |
| `%m`  | Zero-padded month (`01`–`12`)         |
| `%d`  | Zero-padded day (`01`–`31`)           |
| `%H`  | Zero-padded hour, 24-hour (`00`–`23`) |
| `%I`  | Zero-padded hour, 12-hour (`01`–`12`) |
| `%M`  | Zero-padded minute (`00`–`59`)        |
| `%S`  | Zero-padded second (`00`–`59`)        |
| `%p`  | `AM` / `PM`                           |
| `%%`  | Literal `%`                           |

## Features

| Feature        | Description                                                                          |
| -------------- | ------------------------------------------------------------------------------------ |
| `backend-json` | **Default.** JSON locale files via `serde_json`.                                     |
| `backend-po`   | GNU gettext PO locale files (`.po`).                                                 |
| `backend-mo`   | Compiled GNU gettext locale files (`.mo`).                                           |
| `backend-yaml` | YAML locale files (`.yaml`, `.yml`) via `yaml-rust2`.                                |
| `backend-toml` | TOML locale files (`.toml`) via `toml`.                                              |
| `backend-fluent` | Fluent locale files (`.ftl`, simple messages) via `fluent-syntax`.                 |
| `alloc`        | `String` / `Vec` returning helpers (`format_unix_timestamp`, `format_ranges`, ...)   |
| `std`          | Enables `alloc` and runtime locale getters (`get_locale_build()`, ...)               |

### Backend selection

By default the JSON backend is enabled. To use another format, disable default features:

```toml
[dependencies]
embedded-i18n = { version = "0.1", default-features = false, features = ["backend-po"] }
```

Several backends can be enabled simultaneously, the first matching file wins (see [Locale resolution](#locale-resolution)). All parsing happens in the proc macro at compile time: the backends add no code and no dependency to the firmware.

## `no_std` Support

This crate is `#![no_std]` and needs no allocator by default. The `translate!()` macro, the Unicode range helpers and the timestamp formatting all work without `alloc`: they return `&'static` data, work in place or write into a `core::fmt::Write`. Enable the `alloc` feature for the `String` / `Vec` returning variants.

## Minimum Supported Rust Version

Rust 1.85 (edition 2024). Raising it is not considered a breaking change.

## License

Licensed under the <a href="LICENSE">MIT License</a>.

---

Developed by [Alix ANNERAUD](https://alix.anneraud.fr).
