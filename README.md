# embedded-i18n

[![crates.io](https://img.shields.io/crates/v/embedded-i18n.svg)](https://crates.io/crates/embedded-i18n)
[![docs.rs](https://img.shields.io/docsrs/embedded-i18n)](https://docs.rs/embedded-i18n)
[![codecov](https://codecov.io/github/AlixANNERAUD/embedded-i18n/graph/badge.svg?token=0C27GIS0BK)](https://codecov.io/github/AlixANNERAUD/embedded-i18n)

Compile-time internationalization (i18n) for embedded and `no_std` systems. Translates strings at build time from JSON locale files and embeds them directly into ROM, no runtime loading, no allocator required for translations.

## Installation

```toml
[dependencies]
embedded-i18n = "0.1"
```

## Quick Start

1. Place `locales/<locale>.json` files next to your `Cargo.toml`.
2. Set `EMBEDDED_I18N_LOCALE` (and optionally `EMBEDDED_I18N_FALLBACK`) at build time.
3. Use the `translate!()` macro, it resolves to a `&'static str` at compile time.

```rust
use embedded_i18n::translate;

let greeting = translate!("Hello, world!");
```

### Locale files

```json
{
  "Hello, world!": "Bonjour le monde !",
  "Missing argument: {}": "Argument manquant : {}"
}
```

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

Panics at compile time if the key is not found in either the selected locale or the fallback locale.

### Locale resolution

Translations are loaded from `locales/<locale>.json` at compile time. The locale and fallback are selected via environment variables:

| Env variable             | Default | Description                                                    |
| ------------------------ | ------- | -------------------------------------------------------------- |
| `EMBEDDED_I18N_LOCALE`   | `en`    | Primary locale                                                 |
| `EMBEDDED_I18N_FALLBACK` | `en`    | Fallback locale (keys missing from primary are looked up here) |

### Unicode range helpers

Map locales to Unicode code-point ranges for font subsetting:

```rust
use embedded_i18n::{get_locale_ranges, merge_contiguous_ranges, format_ranges};

// Get ranges for a locale
let ranges = get_locale_ranges("ja").unwrap();
// -> [Basic Latin, Hiragana, Katakana, CJK Unified Ideographs]

// Merge overlapping/adjacent ranges
let merged = merge_contiguous_ranges(ranges.to_vec());

// Format for fontconfig-like syntax
let s = format_ranges(merged.iter());
// -> "32-126,12353-12438,12449-12538,19968-40958"
```

Supported locales: `en`, `fr`, `de`, `es`, `it`, `pt`, `nl`, `sv`, `no`, `da`, `pl`, `cs`, `sk`, `hu`, `ro`, `tr`, `ru`, `uk`, `be`, `el`, `ar`, `he`, `ja`, `zh`, `zh-CN`, `zh-TW`, `ko`, `hi`, `th`, `vi`.

### Timestamp formatting

`strftime`-style formatting without libc:

```rust
use embedded_i18n::format_unix_timestamp;

let s = format_unix_timestamp(0, "%Y-%m-%d %H:%M:%S");
// -> "1970-01-01 00:00:00"
```

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

| Feature     | Description                                                                          |
| ----------- | ------------------------------------------------------------------------------------ |
| _(default)_ | `no_std`, `translate!()` macro, unicode ranges, timestamp formatting                 |
| `std`       | Enables runtime locale getters (`get_locale_build()`, `get_fallback_locale_build()`) |

## `no_std` Support

This crate is `#![no_std]`-compatible by default. The `translate!()` macro, unicode range helpers, and timestamp formatting all work without an allocator. The `alloc` crate is only required for `String`-returning helpers (`format_unix_timestamp`, `format_ranges`).

## License

Licensed under the <a href="LICENSE">MIT License</a>.

---

Developed by [Alix ANNERAUD](https://alix.anneraud.fr).
