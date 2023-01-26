#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use nym_connect_android::AppBuilder;

fn main() {
    // As per breaking change description here
    // https://github.com/tauri-apps/tauri/blob/feac1d193c6d618e49916ad0707201f43d5cdd36/tooling/bundler/CHANGELOG.md
    if let Err(error) = fix_path_env::fix() {
        log::warn!("Failed to fix PATH: {error}");
    }

    AppBuilder::new().run();
}
