# `no_std` Support

`embedded-i18n` is `#![no_std]` and needs no allocator by default.
All the parsing happens in the procedural macros, on the build machine: the firmware only contains string literals and, for plurals, selects and runtime locales, a few comparisons.

| Part | Needs |
| --- | --- |
| [`translate!`], plural, select and context | nothing: `&'static str` or `&'static CStr` literals |
| [`translate_write!`], [`translate_args!`] and the write variants | [`core::fmt`] |
| [Runtime locale selection][guide-runtime-locales] | an `AtomicU8` with load and store, no compare-and-swap |
| [Date and time][guide-date-time], [Unicode ranges][guide-unicode-ranges] | [`core::fmt`], writing into a [`core::fmt::Write`] |
| `String` and `Vec` returning helpers | the `alloc` feature |
| Build environment getters | the `std` feature |

Writing a translated message into a fixed-size buffer, without allocator:

```rust
use core::fmt::{self, Write};
use embedded_i18n::translate_plural_write;

struct Buffer {
    bytes: [u8; 32],
    length: usize,
}

impl Write for Buffer {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.length + text.len();
        self.bytes.get_mut(self.length..end).ok_or(fmt::Error)?.copy_from_slice(text.as_bytes());
        self.length = end;
        Ok(())
    }
}

let mut buffer = Buffer { bytes: [0; 32], length: 0 };
translate_plural_write!(&mut buffer, "files", 3)?;
assert_eq!(&buffer.bytes[..buffer.length], b"3 files");
# Ok::<(), fmt::Error>(())
```

Crates such as `heapless` provide ready-made fixed-capacity strings implementing [`core::fmt::Write`].

## Features

| Feature | Description |
| --- | --- |
| `backend-json` (default), `backend-yaml`, `backend-toml`, `backend-po`, `backend-mo`, `backend-fluent` | Locale file formats, see [Locale Files][guide-locale-files]. They only affect the procedural macros and add nothing to the firmware. |
| `alloc` | `String` / `Vec` returning helpers: `format_unix_timestamp`, `DateTimeFormatter::format`, `merge_contiguous_ranges`, `format_range`, `format_ranges`. |
| `std` | Enables `alloc`, and `get_locale_build` / `get_fallback_locale_build` for build scripts. |

## Targets

CI builds the crate for `thumbv7em-none-eabihf` (Cortex-M4F) with the JSON and PO backends, with and without `alloc`.
The runtime locale selection only uses atomic loads and stores, so it also builds for targets without compare-and-swap, such as `thumbv6m-none-eabi` (Cortex-M0).
Any target with `core` and 8-bit atomic loads and stores works.

[guide-runtime-locales]: runtime_locales.md
[guide-date-time]: date_time.md
[guide-unicode-ranges]: unicode_ranges.md
[guide-locale-files]: locale_files.md
[`translate!`]: translating.md
[`translate_write!`]: formatting.md
[`translate_args!`]: formatting.md
[`core::fmt::Write`]: https://doc.rust-lang.org/core/fmt/trait.Write.html
[`core::fmt`]: https://doc.rust-lang.org/core/fmt/index.html
