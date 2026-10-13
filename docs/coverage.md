# Translation Coverage

By default, a message missing from the selected locale silently resolves through the fallback locales.
That is what you want while translations are in progress, but it can ship a French firmware with English strings.

## Strict Mode

Build with `EMBEDDED_I18N_STRICT=1` to forbid falling back:

```sh
EMBEDDED_I18N_STRICT=1 EMBEDDED_I18N_LOCALE=fr cargo build --release
```

Every [`translate!`], [`translate_plural!`] and [`translate_select!`] (and their write and args variants) must then find its message in the primary locale, i.e. the selected locale and its less specific forms (`fr-CA` and `fr` for `EMBEDDED_I18N_LOCALE=fr-CA`), otherwise the build fails at the call site:

```text
error: translation for "Settings" is missing from the primary locale "fr" (EMBEDDED_I18N_STRICT is set)
```

Every untranslated call site is reported in one build, not only the first one.
Entries ignored by the parsers count as missing: empty translations, fuzzy PO entries, plural messages with an empty form.
With [several embedded locales][guide-runtime-locales], every embedded locale must be complete.
The [pseudo-locale][guide-pseudo-localization] is always complete.

A CI job building each shipped locale in strict mode catches untranslated strings before release:

```yaml
strategy:
  matrix:
    locale: [en, fr, de]
steps:
  - run: cargo build --release
    env:
      EMBEDDED_I18N_LOCALE: ${{ matrix.locale }}
      EMBEDDED_I18N_STRICT: "1"
```

## Key Extraction and Unused Keys

There is no built-in extraction of the keys used in the source code (an `xgettext` equivalent), nor a report of keys present in locale files but never used.
A procedural macro only sees its own call site, cannot know when the last call site of a crate has been expanded, and cannot emit warnings on stable Rust, so these reports would need a separate tool scanning the sources.
Strict mode covers the most harmful case, untranslated strings, at compile time.

[guide-runtime-locales]: runtime_locales.md
[guide-pseudo-localization]: pseudo_localization.md
[`translate!`]: translating.md
[`translate_plural!`]: plurals.md
[`translate_select!`]: select.md
