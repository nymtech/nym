#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use nym_connect::AppBuilder;

fn main() {
    AppBuilder::new().run();
}
