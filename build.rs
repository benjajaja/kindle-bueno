use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let absolute_path = PathBuf::from(manifest_dir).join("sensitive");

    println!("cargo:rustc-env=CONFIG_DIR={}", absolute_path.display());

    let path_str = std::fs::read_to_string("sensitive/logo").expect("Failed to read config file");
    let path_str = path_str.trim();

    let bytes = std::fs::read(format!("src/{path_str}"))
        .expect(&format!("Failed to read bytes file {}", path_str));

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::PathBuf::from(&out_dir).join("logo.rs");
    std::fs::write(&out_path, format!("pub const LOGO: &[u8] = &{:?};", bytes)).unwrap();

    println!("cargo:rustc-env=OUT_LOGO={}", out_path.display());
}
