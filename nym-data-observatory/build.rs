// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use glob::glob;
use std::env;
use std::path::Path;

fn main() {
    // copy common manifest files from "../common/nyxd-scraper-psql/sql_migrations/* migrations"
    println!("Copying common migrations...");
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let common_migrations_path =
        Path::new(&manifest_dir_string).join("../common/nyxd-scraper-psql/sql_migrations/");
    let output_path = Path::new(&manifest_dir_string).join("migrations");
    println!(
        "output_path: {:?} (exists = {})",
        output_path,
        output_path.exists()
    );
    let common_migrations_path = common_migrations_path.as_path();
    println!(
        "common_migrations_path: {:?} (exists = {})",
        common_migrations_path,
        common_migrations_path.exists()
    );
    for file in glob(&format!("{common_migrations_path:?}/*"))
        .unwrap()
        .flatten()
    {
        println!("- {file:?}");
        std::fs::copy(file, &output_path).unwrap();
    }

    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        println!("cargo:rustc-env=DATABASE_URL={database_url}");
    }
}
