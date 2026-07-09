fn main() {
    println!("cargo:rerun-if-env-changed=EMBEDDED_I18N_LOCALE");
    println!("cargo:rerun-if-env-changed=EMBEDDED_I18N_FALLBACK");
}
