# Message Context

The same source string can need different translations: `Open` as a menu action is `Ouvrir` in French, but `Open` as a state is `Ouvert`.
A message context disambiguates them.
Pass `context = "..."` before the key:

```rust
use embedded_i18n::translate;

assert_eq!(translate!(context = "menu", "Open"), "Open (menu)");
assert_eq!(translate!(context = "state", "Open"), "Open (state)");
```

`context` is accepted by every macro, works with C string keys, and combines with [plural forms][guide-plurals]:

```rust
use embedded_i18n::translate_plural;

assert_eq!(translate_plural!(context = "menu", "files", 2), "Open files");
```

## In Locale Files

GNU gettext PO and MO files use `msgctxt`:

```po
msgctxt "menu"
msgid "Open"
msgstr "Ouvrir"

msgctxt "state"
msgid "Open"
msgstr "Ouvert"
```

JSON, YAML, TOML and Fluent files use a namespace named after the context, i.e. the key `menu.Open`:

```json
{
  "menu": { "Open": "Ouvrir" },
  "state": { "Open": "Ouvert" }
}
```

A message with a context never falls back to the same key without context: if `menu.Open` is missing from every locale of the chain, `translate!(context = "menu", "Open")` is a compile error, even if `Open` exists.

[guide-plurals]: plurals.md
