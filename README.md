# embedded-i18n

Compile-time internationalization (i18n) for embedded and `no_std` systems. Translates strings at build time from JSON locale files and embeds them directly into ROM — no runtime loading, no allocator required for translations.

## How it works

1. Place `locales/<locale>.json` files next to your `Cargo.toml` (where `<locale>` is e.g. `en`, `fr`, `ja`).
2. Set `EMBEDDED_I18N_LOCALE` and optionally `EMBEDDED_I18N_FALLBACK` environment variables at build time.
3. Use the `translate!()` macro — it resolves to a `&'static str` (or `&'static CStr` with `c"..."`) at compile time.

## Usage

```rust
use embedded_i18n::translate;

// Basic string translation
let msg = translate!("Hello, world!");

// C string variant (for FFI)
let c_msg = translate!(c"Hello, world!");
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

Or set them in `config.toml` / `build.rs`.

## Features

- `std` — enables runtime locale getters (`get_locale_build()`, `get_fallback_locale_build()`)
- Default (`no_std`) — const fn locale getters via `option_env!`

## Unicode range helpers

Map locales to Unicode code-point ranges for font subsetting:

```rust
use embedded_i18n::get_locale_ranges;

let ranges = get_locale_ranges("ja").unwrap();
// -> [Basic Latin, Hiragana, Katakana, CJK Unified Ideographs]
```

## Timestamp formatting

`strftime`-style formatting without libc:

```rust
use embedded_i18n::format_unix_timestamp;

let s = format_unix_timestamp(0, "%Y-%m-%d %H:%M:%S");
// -> "1970-01-01 00:00:00"
```

## License

GPL-2.0-only
