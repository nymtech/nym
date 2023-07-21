use std::env;

mod constants;

use constants::{SENTRY_DSN_JS, SENTRY_DSN_RUST};

fn main() {
    // set these env vars at compile time
    if let Ok(dsn) = env::var(SENTRY_DSN_RUST) {
        println!("cargo:rustc-env={}={}", SENTRY_DSN_RUST, dsn);
    }
    if let Ok(dsn) = env::var(SENTRY_DSN_JS) {
        println!("cargo:rustc-env={}={}", SENTRY_DSN_JS, dsn);
    }
    tauri_build::build();
}
