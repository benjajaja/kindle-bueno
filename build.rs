use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=PLAYA_BLANCA");

    let config_dir = if env::var("PLAYA_BLANCA").is_ok() {
        "sensitive/playa_blanca"
    } else {
        "sensitive"
    };

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let absolute_path = PathBuf::from(manifest_dir).join(config_dir);

    println!("cargo:rustc-env=CONFIG_DIR={}", absolute_path.display());
}
