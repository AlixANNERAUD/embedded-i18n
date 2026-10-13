# Formatting

Translations often contain placeholders: `"Hello {name}, you have {count} messages"`.
Since a translation expands to a string literal, it can be the format string of the formatting macros, so placeholders are checked against the arguments at compile time, in every language you build, and formatting needs no allocation.

## Writing Into a Buffer

[`translate_write!`] expands to `write!(writer, <translation>, arguments...)` and returns [`core::fmt::Result`].
It works with any [`core::fmt::Write`]: a display driver, a fixed-size buffer, a UART, or a `String`:

```rust
use core::fmt::Write;
use embedded_i18n::translate_write;

let mut output = String::new();
translate_write!(&mut output, "with_arg", 42)?;
assert_eq!(output, "Value: 42");
# Ok::<(), core::fmt::Error>(())
```

Positional (`{}`), named (`{name}` with `name = value`) and implicitly captured placeholders (`{name}` with a `name` variable in scope) are supported:

```rust
use core::fmt::Write;
use embedded_i18n::translate_write;

let name = "Ada";
let count = 3;

let mut output = String::new();
translate_write!(&mut output, "with_named_args")?;
assert_eq!(output, "Hello Ada, you have 3 messages");

output.clear();
translate_write!(&mut output, "with_named_args", name = "Grace", count = 1)?;
assert_eq!(output, "Hello Grace, you have 1 messages");
# Ok::<(), core::fmt::Error>(())
```

Like `write!`, the writer only needs a `write_fmt` method, so `std::io::Write` works too.

## Format Arguments

[`translate_args!`] expands to `format_args!(<translation>, arguments...)` and returns [`core::fmt::Arguments`], usable wherever a `format_args!` is: logging macros (`defmt` excluded, see below), `panic!`, or a `Display` field:

```rust
use embedded_i18n::translate_args;

let name = "Ada";
let text = format!("> {}", translate_args!("with_named_args", count = 3));
assert_eq!(text, "> Hello Ada, you have 3 messages");
```

`format_args!` borrows its arguments, so its result must be used in the same expression.

## With an Allocator

With `alloc` or `std`, `format!` accepts a translation as its format string, but only with explicit arguments: implicit captures are not visible to `format!` through another macro.

```rust
use embedded_i18n::translate;

let text = format!(translate!("with_named_args"), name = "Ada", count = 3);
assert_eq!(text, "Hello Ada, you have 3 messages");
```

```rust,compile_fail
use embedded_i18n::translate;

let name = "Ada";
let count = 3;
let text = format!(translate!("with_named_args"));
```

Prefer [`translate_write!`] or [`translate_args!`], which support captures and keep working when [several locales are embedded][guide-runtime-locales].

## `defmt`

`defmt` interns format strings at compile time and needs them as literal tokens, so translations cannot be `defmt` format strings.
Log the translation as an argument instead: `defmt::info!("{=str}", translate!("greeting"))`.

## Compile Errors

The standard format checks apply to the translation of every locale you build:

- a placeholder without an argument, e.g. a translator writing `{nom}` instead of `{name}`;
- an argument not used by the translation;
- invalid format syntax, such as an unescaped `{` (write `{{`).

Building each shipped locale in CI catches these mistakes before release.

[guide-runtime-locales]: runtime_locales.md
[`translate_write!`]: formatting.md
[`translate_args!`]: formatting.md
[`core::fmt::Write`]: https://doc.rust-lang.org/core/fmt/trait.Write.html
[`core::fmt::Arguments`]: https://doc.rust-lang.org/core/fmt/struct.Arguments.html
[`core::fmt::Result`]: https://doc.rust-lang.org/core/fmt/type.Result.html
