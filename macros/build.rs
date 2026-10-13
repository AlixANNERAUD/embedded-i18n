fn main() {
    println!("cargo:rerun-if-env-changed=EMBEDDED_I18N_LOCALE");
    println!("cargo:rerun-if-env-changed=EMBEDDED_I18N_FALLBACK");
    println!("cargo:rerun-if-env-changed=EMBEDDED_I18N_PATH");
    println!("cargo:rerun-if-env-changed=EMBEDDED_I18N_STRICT");
    println!("cargo:rerun-if-env-changed=EMBEDDED_I18N_BIDI_ISOLATION");
}
