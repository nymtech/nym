use std::env;

fn main() {
    // Enable debug output during build
    if env::var("CARGO_FEATURE_DEBUG_INVENTORY").is_ok() || env::var("DEBUG_HTTP_INVENTORY").is_ok()
    {
        println!("cargo:warning=HTTP Client Inventory Debug Enabled");
        println!("cargo:rustc-cfg=debug_inventory");
    }

    // Force rebuild when this environment variable changes
    println!("cargo:rerun-if-env-changed=DEBUG_HTTP_INVENTORY");
}
