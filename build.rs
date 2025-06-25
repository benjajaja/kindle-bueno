use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=PLAYA_BLANCA");

    let config_dir = if env::var("PLAYA_BLANCA").is_ok() {
        "sensitive/playa_blanca"
    } else {
        "sensitive"
    };

    println!("cargo:rustc-env=CONFIG_DIR={}", config_dir);
}
