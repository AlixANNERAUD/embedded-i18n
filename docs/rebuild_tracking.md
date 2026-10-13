# Rebuild Tracking

Cargo rebuilds a crate when its sources change, but it does not know that the translation macros read the locale files.
Without tracking, editing `locales/fr.json` and running `cargo build` again keeps the old strings.

Invoke [`track_locales!`] once in each crate using the macros, at module level, typically in `main.rs` or `lib.rs`:

```rust
embedded_i18n::track_locales!();
# fn main() {}
```

It expands to an unused constant with an `include_bytes!` of every locale file loaded for the build, so Cargo rebuilds the crate whenever one of them changes.
Nothing is added to the binary.

## What Triggers a Rebuild

| Change | Rebuild |
| --- | --- |
| Editing a loaded locale file | with [`track_locales!`] |
| Changing an `EMBEDDED_I18N_*` variable | always |
| Creating a locale file that did not exist at the previous build | no, run `cargo clean -p <crate>` or touch a source file |
| Enabling another backend feature | always |

A locale file created after the build, such as a new `fr-CA.json` next to an existing `fr.json`, is not seen until the crate is rebuilt for another reason, because only existing files can be included.

## Tracking a Whole Directory

To also pick up new files, add a build script to the crate using the macros:

```rust,no_run
fn main() {
    println!("cargo:rerun-if-changed=locales");
}
```

Cargo then reruns the build script, and rebuilds the crate, whenever a file of `locales/` is added, removed or modified.
Adjust the path if you set `EMBEDDED_I18N_PATH`.

[`track_locales!`]: rebuild_tracking.md
