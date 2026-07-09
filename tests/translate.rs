use embedded_i18n::translate;

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
