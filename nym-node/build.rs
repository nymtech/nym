// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::PathBuf;

// that's disgusting, but it works, so it's good enough for now ¯\_(ツ)_/¯
fn main() {
    let out_dir: PathBuf = std::env::var("OUT_DIR").unwrap().into();

    let path: PathBuf = std::env::var("CARGO_MANIFEST_DIR").unwrap().into();

    let mix_path = path.parent().unwrap().join("mixnode");
    let gateway_path = path.parent().unwrap().join("gateway");

    let mix_meta = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .current_dir(&mix_path)
        .exec()
        .unwrap();
    let mix_version = &mix_meta.root_package().unwrap().version;

    let gateway_meta = MetadataCommand::new()
        .manifest_path("./Cargo.toml")
        .current_dir(&gateway_path)
        .exec()
        .unwrap();

    let gateway_version = &gateway_meta.root_package().unwrap().version;

    fs::write(out_dir.join("mixnode_version"), mix_version.to_string()).unwrap();
    fs::write(out_dir.join("gateway_version"), gateway_version.to_string()).unwrap();

    println!("cargo::rerun-if-changed=build.rs");
}
