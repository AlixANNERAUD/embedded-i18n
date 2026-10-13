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

Simple Fluent messages are supported: text, message and term references, string and number literals, and attributes (as `login.placeholder`). Variables become named format placeholders (`{ $user-name }` → `{user_name}`) for `translate_args!()` and `translate_write!()`. A select expression making up a whole message becomes one key per variant (`emails.one`, `emails.other`) and the default variant is also available as `emails`, which is how plural forms are written in Fluent. Select expressions mixed with text, functions and parameterized terms are reported as compile errors.

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

### Plural forms

`translate_plural!()` picks the plural form matching a runtime count, using the plural rules of the locale the message comes from. Only the selection runs at runtime: an `if` / `match` on the count, generated at compile time, returning a `&'static str` (or `&'static CStr`).

```rust
use embedded_i18n::{translate_plural, translate_plural_write};

let label: &'static str = translate_plural!("files", count);

translate_plural_write!(&mut display, "files", count)?;
```

`translate_plural_write!()` writes the selected form into any `core::fmt::Write` and binds the count to `count`, so forms can use `{count}` (and any variable in scope) without allocating. Extra format arguments can follow the count, but every form must then use them. Both macros accept a `context` and any integer count; negative counts use their absolute value.

**JSON, YAML, TOML and Fluent** use the [CLDR plural categories](https://cldr.unicode.org/index/cldr-spec/plural-rules) as nested keys:

```json
{
  "files": {
    "zero": "No files",
    "one": "{count} file",
    "other": "{count} files"
  }
}
```

Every category the locale uses for integers is required, plus `other` (e.g. `one`, `few`, `many` and `other` for Russian), and a missing one is a compile error. `zero` is optional and used for a count of 0 in languages without a `zero` category. The `many` form of French, Spanish, Italian, Portuguese and Catalan (millions) is optional and falls back to `other`. Integer plural rules are built in for `ar`, `be`, `bg`, `bn`, `ca`, `cs`, `da`, `de`, `el`, `en`, `es`, `et`, `fi`, `fr`, `he`, `hi`, `hu`, `id`, `it`, `ja`, `km`, `ko`, `lo`, `ms`, `my`, `nb`, `nl`, `nn`, `no`, `pl`, `pt`, `pt-PT`, `ro`, `ru`, `sk`, `sv`, `th`, `tr`, `uk`, `vi` and `zh`.

**GNU gettext PO / MO** use `msgid_plural` and `msgstr[N]`, and the `Plural-Forms` expression of the file header is compiled to Rust, so any language works. Without a header the gettext default `plural=(n != 1)` applies.

```po
msgid ""
msgstr ""
"Plural-Forms: nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);\n"

msgid "file"
msgid_plural "files"
msgstr[0] "{count} файл"
msgstr[1] "{count} файла"
msgstr[2] "{count} файлов"
```

`translate!()` on a gettext plural message returns its first form. As with `msgfmt`, a plural entry with an empty form is treated as untranslated and resolves through the fallback locales.

### Select variants

`translate_select!()` chooses between variants of a message from a runtime value other than a count: gender, device state, enum variant... The value can be anything implementing `AsRef<str>`, and the expansion is a `match` on string literals returning a `&'static str` (or `&'static CStr`), with no allocation.

```rust
use embedded_i18n::{translate_select, translate_select_write};

let reply = translate_select!("replied", gender);

translate_select_write!(&mut display, "replied", user.gender, name = user.name)?;
```

Variants are nested keys, and an `other` variant is required: it is used for any value without a variant of its own.

```json
{
  "replied": {
    "male": "He replied",
    "female": "She replied",
    "other": "They replied"
  }
}
```

In PO files the variants are flat keys (`msgid "replied.male"`), and in Fluent a select expression making up the whole message (`{ $gender -> [male] ... *[other] ... }`). All variants come from the first locale of the chain defining an `other` variant for the message. Like the other macros, both accept a `context`, and `translate_select_write!()` takes extra format arguments after the value.

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
| `EMBEDDED_I18N_LOCALES`  | unset   | Comma separated locales to embed for runtime selection (see [Runtime locale selection](#runtime-locale-selection)) |
| `EMBEDDED_I18N_FALLBACK` | `en`    | Comma separated fallback locales (keys missing from the primary locale are looked up here, in order) |
| `EMBEDDED_I18N_PATH`     | `locales` | Locale files directory, relative to the `Cargo.toml` of the crate calling `translate!()`, or absolute |
| `EMBEDDED_I18N_BIDI_ISOLATION` | unset | When set, placeholders of right-to-left locales are wrapped in Unicode bidi isolates (see [Right-to-left languages](#right-to-left-languages)) |
| `EMBEDDED_I18N_STRICT`   | unset   | When set (`1`, `true`, ...), a message missing from the primary locale is a compile error instead of using the fallbacks |

Locale identifiers are normalized like BCP 47 tags (`fr_ca`, `FR-CA` and `fr-CA` are the same locale), and region or script subtags are dropped one at a time before moving to the next fallback. With `EMBEDDED_I18N_LOCALE=fr-CA` and `EMBEDDED_I18N_FALLBACK=de,en`, a key is looked up in `fr-CA`, `fr`, `de`, then `en`. Locale files can be named `fr-CA`, `fr_CA`, `fr-ca` or `fr_ca`. Missing files are skipped and the first non-empty translation wins.

To share one set of locale files between the crates of a workspace, set the path once in `.cargo/config.toml` at the workspace root. `relative = true` makes it relative to that file:

```toml
[env]
EMBEDDED_I18N_PATH = { value = "i18n", relative = true }
```

A missing directory is reported as a compile error naming the resolved path.

### Runtime locale selection

By default a firmware contains one locale and every macro expands to a literal. To let the user pick a language at runtime (a settings menu, a DIP switch...), list the locales to embed:

```sh
EMBEDDED_I18N_LOCALES=en,fr,de EMBEDDED_I18N_LOCALE=en cargo build
```

```rust
use embedded_i18n::{locale, set_locale, translate, LOCALES};

assert_eq!(LOCALES, ["en", "fr", "de"]);
assert_eq!(translate!("greeting"), "Hello");

set_locale("fr")?;
assert_eq!(locale(), "fr");
assert_eq!(translate!("greeting"), "Bonjour");
```

- Each embedded locale is resolved at compile time with its own fallback chain (`EMBEDDED_I18N_FALLBACK`), and every message must be found for every embedded locale, otherwise the build fails and lists each locale missing it.
- `EMBEDDED_I18N_LOCALE` selects the locale active at startup (the first embedded one if it is not listed).
- `set_locale()` / `set_locale_index()` change the current locale, `locale()` / `locale_index()` read it, `LOCALES` lists the embedded locales. The current locale is an `AtomicU8` used with plain loads and stores, so it works on targets without compare-and-swap (e.g. `thumbv6m`) and needs no allocation or critical section.
- `translate!()` expands to an index into a static array with one entry per locale. The plural, select and write macros expand to a `match` on the locale index, and `translate_args!()` formats through a closure (formatting flags such as width are not applied to the whole message).
- `translate!()` is then a runtime value: it cannot be used in `const` items or as the format string of `format!` / `write!`; use `translate_args!()` and `translate_write!()` instead.
- `DATE_TIME_LOCALE` keeps following `EMBEDDED_I18N_LOCALE`; use `date_time_locale(locale())` to format dates in the current locale (this links the date and time names of every supported locale).

ROM cost: every embedded locale adds its strings for each message used (identical strings are usually merged by the linker), plus, per `translate!()` call site, a table of one string reference per locale (2 words each) and a few instructions to index it. The other macros cost one comparison per embedded locale at each call site. With a single embedded locale (`EMBEDDED_I18N_LOCALES=fr`), the expansion stays a literal.

### Translation coverage

By default a message missing from the selected locale silently resolves through the fallback locales, which is what you want while translations are in progress. To make sure a release ships fully translated, build with strict mode:

```sh
EMBEDDED_I18N_STRICT=1 EMBEDDED_I18N_LOCALE=fr cargo build
```

Every `translate!()`, `translate_plural!()` and `translate_select!()` must then find its message in the primary locale (`fr-CA` and `fr` for `EMBEDDED_I18N_LOCALE=fr-CA`), otherwise the build fails at the call site:

```text
error: translation for "Settings" is missing from the primary locale "fr" (EMBEDDED_I18N_STRICT is set)
```

Running a strict build for each shipped locale in CI catches untranslated strings before release.

### Right-to-left languages

Interpolating a left-to-right value (a name, a number, a file name) into Arabic or Hebrew text can reorder the surrounding words. With `EMBEDDED_I18N_BIDI_ISOLATION=1`, every `{...}` placeholder of a message coming from a right-to-left locale is wrapped at compile time in Unicode isolates (U+2068 FIRST STRONG ISOLATE and U+2069 POP DIRECTIONAL ISOLATE), as Fluent does. It is off by default because many embedded text renderers do not implement the Unicode bidi algorithm and would draw the isolates as missing glyphs.

The layout direction is available at compile time to mirror the UI:

```rust
use embedded_i18n::{is_locale_right_to_left, is_right_to_left};

const MIRRORED: bool = is_locale_right_to_left();

assert!(is_right_to_left("ar-EG"));
assert!(is_right_to_left("ku-Arab"));
```

`is_right_to_left()` recognizes right-to-left languages (`ar`, `he`, `fa`, `ur`, `ps`, `sd`, `ug`, `yi`, ...) and the `Arab` / `Hebr` script subtags.

### Pseudo-localization

`EMBEDDED_I18N_LOCALE=pseudo` builds a pseudo-locale from the fallback locales, to check layouts for truncation, hardcoded strings and missing glyphs before real translations exist:

```sh
EMBEDDED_I18N_LOCALE=pseudo EMBEDDED_I18N_FALLBACK=en cargo run
```

```text
Hello {name}  →  [Hélló {name} ~~]
```

Letters are accented with Latin-1 characters, the text is padded by about a third with `~` and wrapped in brackets, so clipped text is easy to spot. Placeholders and `{{` / `}}` escapes are preserved, so `translate_args!()`, `translate_write!()` and the plural and select macros keep working. Strict mode accepts every message of the pseudo-locale.

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

### Localized date and time formatting

`strftime`-style formatting without libc and without allocation, written into any `core::fmt::Write`. Month and weekday names, AM/PM markers and the default date and time patterns follow the build locale:

```rust
use embedded_i18n::{write_unix_timestamp, DateTimeFormatter};

write_unix_timestamp(&mut output, 1_791_884_800, "%A %d %B %Y, %X")?;
// EMBEDDED_I18N_LOCALE=en -> "Tuesday 13 October 2026, 09:46:40 AM"
// EMBEDDED_I18N_LOCALE=fr -> "mardi 13 octobre 2026, 09:46:40"

let paris = DateTimeFormatter::new().with_utc_offset(2 * 3600);
paris.write(&mut output, 1_791_884_800, "%x %H:%M %z")?;
// EMBEDDED_I18N_LOCALE=fr -> "13/10/2026 11:46 +0200"

write!(output, "Updated {}", paris.display(1_791_884_800, "%c"))?;
```

With the `alloc` feature, `format_unix_timestamp()` and `DateTimeFormatter::format()` return a `String`.

| Token       | Description                                                  |
| ----------- | ------------------------------------------------------------ |
| `%Y`        | Year with century (e.g. `2026`)                              |
| `%y`        | Year without century (`00`–`99`)                             |
| `%m`        | Zero-padded month (`01`–`12`)                                |
| `%d`        | Zero-padded day (`01`–`31`)                                  |
| `%e`        | Space-padded day (` 1`–`31`)                                 |
| `%H`        | Zero-padded hour, 24-hour (`00`–`23`)                        |
| `%I`        | Zero-padded hour, 12-hour (`01`–`12`)                        |
| `%M`        | Zero-padded minute (`00`–`59`)                               |
| `%S`        | Zero-padded second (`00`–`59`)                               |
| `%p`        | Localized AM / PM marker (`AM`, `午後`, `오전`, ...)            |
| `%B`        | Localized month name, in the form used in dates (`октября`)  |
| `%OB`       | Localized standalone month name (`октябрь`)                  |
| `%b` / `%h` | Localized abbreviated month name                             |
| `%A`        | Localized weekday name                                       |
| `%a`        | Localized abbreviated weekday name                           |
| `%u` / `%w` | Weekday number, `1`–`7` from Monday / `0`–`6` from Sunday    |
| `%x`        | Localized date (`10/13/2026`, `13/10/2026`, `13.10.2026`...) |
| `%X`        | Localized time (`09:46:40 AM`, `09:46:40`...)                |
| `%c`        | Localized date and time (`%x %X`)                            |
| `%z`        | UTC offset (`+0200`, `-0330`)                                |
| `%%`        | Literal `%`                                                  |

The names and patterns come from a compile-time table covering `ar`, `be`, `cs`, `da`, `de`, `el`, `en` (with `en-GB`, `en-IE`, `en-AU`, `en-NZ`, `en-IN` and `en-CA` patterns), `es`, `fr` (with `fr-CA` and `fr-CH` patterns), `he`, `hi`, `hu`, `it`, `ja`, `ko`, `nb` / `nn` / `no`, `nl`, `pl`, `pt`, `ro`, `ru`, `sk`, `sv`, `th`, `tr`, `uk`, `vi` and `zh`. The build locale is resolved once at compile time (`EMBEDDED_I18N_LOCALE`, then each `EMBEDDED_I18N_FALLBACK`, then English) into the `DATE_TIME_LOCALE` constant, so only the strings of that locale end up in the firmware. `date_time_locale("de")` returns another locale's data for `DateTimeFormatter::with_locale()`; calling it with a runtime string links the whole table. Digits are always ASCII and the calendar is always Gregorian.

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
