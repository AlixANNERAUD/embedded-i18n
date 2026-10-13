# Date and Time

[`DateTimeFormatter`] formats Unix timestamps with `strftime`-style patterns, without `libc`, without allocation and without a time zone database, into any [`core::fmt::Write`].
Month and weekday names, AM/PM markers and the default date and time patterns follow the build locale:

```rust
use embedded_i18n::write_unix_timestamp;

let mut output = String::new();
write_unix_timestamp(&mut output, 1_791_884_800, "%A %d %B %Y, %X")?;
assert_eq!(output, "Tuesday 13 October 2026, 09:46:40 AM");
// EMBEDDED_I18N_LOCALE=fr: "mardi 13 octobre 2026, 09:46:40"
# Ok::<(), core::fmt::Error>(())
```

## Formatter

A formatter can use the names of another locale and a fixed UTC offset, and is `const`:

```rust
use embedded_i18n::{date_time_locale, DateTimeFormatter};

const PARIS: DateTimeFormatter = DateTimeFormatter::new()
    .with_locale(date_time_locale("fr"))
    .with_utc_offset(2 * 3600);

let mut output = String::new();
PARIS.write(&mut output, 1_791_884_800, "%x %H:%M %z")?;
assert_eq!(output, "13/10/2026 11:46 +0200");

let text = format!("Mis à jour le {}", PARIS.display(1_791_884_800, "%d %B"));
assert_eq!(text, "Mis à jour le 13 octobre");
# Ok::<(), core::fmt::Error>(())
```

- [`DateTimeFormatter::write`] writes into a [`core::fmt::Write`].
- [`DateTimeFormatter::display`] returns a value implementing [`core::fmt::Display`], for `write!`, `format_args!` or logging, still without allocation.
- With the `alloc` feature, [`DateTimeFormatter::format`] and `format_unix_timestamp` return a `String`.

The UTC offset is fixed: daylight saving time and time zone rules are left to the application, which usually gets the offset from the network or a configuration.

## Tokens

| Token | Description |
| --- | --- |
| `%Y` | Year with century (`2026`) |
| `%y` | Year without century (`00`–`99`) |
| `%m` | Zero-padded month (`01`–`12`) |
| `%d` | Zero-padded day (`01`–`31`) |
| `%e` | Space-padded day (` 1`–`31`) |
| `%H` | Zero-padded hour, 24-hour clock (`00`–`23`) |
| `%I` | Zero-padded hour, 12-hour clock (`01`–`12`) |
| `%M` | Zero-padded minute (`00`–`59`) |
| `%S` | Zero-padded second (`00`–`59`) |
| `%p` | Localized AM / PM marker (`AM`, `午後`, `오전`...) |
| `%B` | Localized month name, in the form used in dates (`октября`) |
| `%OB` | Localized standalone month name (`октябрь`) |
| `%b`, `%h` | Localized abbreviated month name |
| `%A` | Localized weekday name |
| `%a` | Localized abbreviated weekday name |
| `%u` | Weekday number from Monday (`1`–`7`) |
| `%w` | Weekday number from Sunday (`0`–`6`) |
| `%x` | Localized date (`10/13/2026`, `13/10/2026`, `13.10.2026`...) |
| `%X` | Localized time (`09:46:40 AM`, `09:46:40`...) |
| `%c` | Localized date and time (`%x %X`) |
| `%z` | UTC offset (`+0200`, `-0330`) |
| `%%` | Literal `%` |

Unknown tokens are written as is (`%Q` stays `%Q`), and a trailing `%` is kept.

Some languages decline month names: Russian writes `13 октября` in a date but `октябрь` alone, which is why `%B` and `%OB` differ:

```rust
use embedded_i18n::{date_time_locale, DateTimeFormatter};

let russian = DateTimeFormatter::new().with_locale(date_time_locale("ru"));
let mut output = String::new();
russian.write(&mut output, 1_791_884_800, "%d %B / %OB")?;
assert_eq!(output, "13 октября / октябрь");
# Ok::<(), core::fmt::Error>(())
```

## Locales

Names and patterns come from a compile-time table covering `ar`, `be`, `cs`, `da`, `de`, `el`, `en`, `es`, `fr`, `he`, `hi`, `hu`, `it`, `ja`, `ko`, `nb`, `nl`, `nn`, `no`, `pl`, `pt`, `ro`, `ru`, `sk`, `sv`, `th`, `tr`, `uk`, `vi` and `zh`, with regional date patterns for `en-GB`, `en-IE`, `en-AU`, `en-NZ`, `en-IN`, `en-CA`, `fr-CA` and `fr-CH`.

The build locale is resolved once, at compile time, into the [`DATE_TIME_LOCALE`] constant: `EMBEDDED_I18N_LOCALE`, then each `EMBEDDED_I18N_FALLBACK`, then English.
Only the strings of that locale end up in the firmware.
[`date_time_locale`] returns the data of another locale (English if it is not covered) and [`find_date_time_locale`] returns `None` instead.
Called with a constant, they are resolved at compile time; called with a runtime string, such as the [current locale][guide-runtime-locales], they link the whole table, about 30 locales of names.

Digits are always ASCII, the calendar is always Gregorian, and weeks are not numbered.

[guide-runtime-locales]: runtime_locales.md
[`DateTimeFormatter`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/struct.DateTimeFormatter.html
[`DateTimeFormatter::write`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/struct.DateTimeFormatter.html#method.write
[`DateTimeFormatter::display`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/struct.DateTimeFormatter.html#method.display
[`DateTimeFormatter::format`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/struct.DateTimeFormatter.html#method.format
[`DATE_TIME_LOCALE`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/constant.DATE_TIME_LOCALE.html
[`date_time_locale`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/fn.date_time_locale.html
[`find_date_time_locale`]: https://docs.rs/embedded-i18n/latest/embedded_i18n/fn.find_date_time_locale.html
[`core::fmt::Write`]: https://doc.rust-lang.org/core/fmt/trait.Write.html
[`core::fmt::Display`]: https://doc.rust-lang.org/core/fmt/trait.Display.html
