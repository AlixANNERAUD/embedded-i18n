use core::fmt::Write;

use embedded_i18n::translate;

embedded_i18n::track_locales!();

#[test]
fn basic_translation() {
    assert_eq!(translate!("greeting"), "Hello");
}

#[test]
fn farewell_translation() {
    assert_eq!(translate!("farewell"), "Goodbye");
}

#[test]
fn translation_with_placeholder() {
    assert_eq!(translate!("with_arg"), "Value: {}");
}

#[test]
fn c_string_translation() {
    let c = translate!(c"c_greeting");
    assert_eq!(c.to_bytes(), b"Hello from C");
    assert_eq!(c.to_bytes_with_nul(), b"Hello from C\0");
}

#[test]
fn c_string_length() {
    let c = translate!(c"c_greeting");
    assert_eq!(c.count_bytes(), 12);
}

#[test]
fn translate_is_str() {
    let s: &'static str = translate!("greeting");
    assert_eq!(s, "Hello");
}

#[cfg(feature = "std")]
#[test]
fn translate_works_with_format_macro() {
    let result = std::format!("{} {}", translate!("greeting"), translate!("farewell"));
    assert_eq!(result, "Hello Goodbye");
}

struct Buffer {
    bytes: [u8; 64],
    length: usize,
}

impl core::fmt::Write for Buffer {
    fn write_str(&mut self, value: &str) -> core::fmt::Result {
        let end = self.length + value.len();
        if end > self.bytes.len() {
            return Err(core::fmt::Error);
        }
        self.bytes[self.length..end].copy_from_slice(value.as_bytes());
        self.length = end;
        Ok(())
    }
}

#[test]
fn translate_write_positional() {
    let mut buffer = Buffer {
        bytes: [0; 64],
        length: 0,
    };
    embedded_i18n::translate_write!(&mut buffer, "with_arg", 42).unwrap();
    assert_eq!(&buffer.bytes[..buffer.length], b"Value: 42");
}

#[test]
fn translate_write_named() {
    let mut buffer = Buffer {
        bytes: [0; 64],
        length: 0,
    };
    let name = "Ada";
    embedded_i18n::translate_write!(&mut buffer, "with_named_args", name = name, count = 3)
        .unwrap();
    assert_eq!(
        &buffer.bytes[..buffer.length],
        b"Hello Ada, you have 3 messages"
    );
}

#[test]
fn translate_write_propagates_errors() {
    let mut buffer = Buffer {
        bytes: [0; 64],
        length: 60,
    };
    assert!(embedded_i18n::translate_write!(&mut buffer, "with_arg", 42).is_err());
}

#[test]
fn translate_args_display() {
    let mut buffer = Buffer {
        bytes: [0; 64],
        length: 0,
    };
    buffer
        .write_fmt(embedded_i18n::translate_args!("with_arg", 7))
        .unwrap();
    assert_eq!(&buffer.bytes[..buffer.length], b"Value: 7");
}

#[test]
fn translate_as_format_string() {
    assert_eq!(format!(translate!("with_arg"), 1), "Value: 1");
    assert_eq!(
        format!(
            "{}",
            embedded_i18n::translate_args!("with_named_args", name = "Bob", count = 0)
        ),
        "Hello Bob, you have 0 messages"
    );
}

#[test]
fn translate_write_captures_variables() {
    let mut buffer = Buffer {
        bytes: [0; 64],
        length: 0,
    };
    let name = "Eve";
    let count = 5;
    embedded_i18n::translate_write!(&mut buffer, "with_named_args").unwrap();
    assert_eq!(
        &buffer.bytes[..buffer.length],
        b"Hello Eve, you have 5 messages"
    );
}

#[test]
fn translate_with_context() {
    assert_eq!(translate!(context = "menu", "Open"), "Open (menu)");
    assert_eq!(translate!(context = "state", "Open"), "Open (state)");
}

#[test]
fn translate_with_context_and_c_string() {
    assert_eq!(
        translate!(context = "menu", c"Open").to_bytes(),
        b"Open (menu)"
    );
}

#[test]
fn translate_write_with_context() {
    let mut buffer = Buffer {
        bytes: [0; 64],
        length: 0,
    };
    embedded_i18n::translate_write!(&mut buffer, context = "state", "Open").unwrap();
    assert_eq!(&buffer.bytes[..buffer.length], b"Open (state)");
}
