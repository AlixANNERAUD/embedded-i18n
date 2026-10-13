# Select Variants

Some messages depend on a runtime value other than a count: a gender, a device state, an enum variant.
[`translate_select!`] chooses between the variants of a message from any value implementing `AsRef<str>`:

```rust
use embedded_i18n::translate_select;

assert_eq!(translate_select!("replied", "female"), "She replied");
assert_eq!(translate_select!("replied", "male"), "He replied");
assert_eq!(translate_select!("replied", "unknown"), "They replied");
```

The expansion is a `match` on string literals returning a `&'static str` (or a `&'static CStr` with a C string key), with no allocation and no table lookup.
The `other` variant is required and used for every value without a variant of its own.

Selecting from an enum only needs a string for each variant:

```rust
use embedded_i18n::translate_select;

enum Gender {
    Female,
    Male,
    Unspecified,
}

impl AsRef<str> for Gender {
    fn as_ref(&self) -> &str {
        match self {
            Gender::Female => "female",
            Gender::Male => "male",
            Gender::Unspecified => "other",
        }
    }
}

assert_eq!(translate_select!("replied", Gender::Female), "She replied");
assert_eq!(translate_select!("replied", Gender::Unspecified), "They replied");
```

[`translate_select_write!`] writes the selected variant into any [`core::fmt::Write`], with extra format arguments after the value:

```rust
use core::fmt::Write;
use embedded_i18n::translate_select_write;

let mut output = String::new();
translate_select_write!(&mut output, "replied", "male")?;
assert_eq!(output, "He replied");
# Ok::<(), core::fmt::Error>(())
```

Both macros accept a [`context`][guide-context].

## In Locale Files

Variants are nested keys, with a mandatory `other`:

```json
{
  "replied": {
    "male": "Il a répondu",
    "female": "Elle a répondu",
    "other": "Iel a répondu"
  }
}
```

In PO and MO files, variants are flat keys:

```po
msgid "replied.female"
msgstr "Elle a répondu"

msgid "replied.other"
msgstr "Iel a répondu"
```

In Fluent, a select expression making up the whole message:

```ftl
replied = { $gender ->
    [male] He replied
    [female] She replied
   *[other] They replied
}
```

All variants come from the first locale of the chain defining a non-empty `other` variant for the message: variants are never mixed between locales.

## Compile Errors

- a message without an `other` variant in any locale of the chain;
- format arguments not used by every variant, in [`translate_select_write!`].

[guide-context]: context.md
[`translate_select!`]: select.md
[`translate_select_write!`]: select.md
[`core::fmt::Write`]: https://doc.rust-lang.org/core/fmt/trait.Write.html
