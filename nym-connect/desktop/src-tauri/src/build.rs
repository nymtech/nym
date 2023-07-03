use std::env;

fn main() {
    if env::var_os("NYM_CONNECT_ENABLE_MEDIUM").is_some() {
        println!("cargo:rustc-cfg=medium_enabled");
    }
    println!("cargo:rerun-if-changed=build.rs");

    tauri_build::build();
}
