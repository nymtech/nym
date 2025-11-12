// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use glob::glob;
use std::env;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // check if migrations in "../common/nyxd-scraper-psql/sql_migrations/* are in "nym-data-observatory/migrations"
    println!("Checking common migrations...");
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let common_migrations_path = Path::new(&manifest_dir_string)
        .join("../common/nyxd-scraper-psql/sql_migrations/")
        .canonicalize()?;
    let output_path = Path::new(&manifest_dir_string)
        .join("migrations")
        .canonicalize()?;
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
    for file in glob(&format!("{}/*", common_migrations_path.to_str().unwrap()))
        .unwrap()
        .flatten()
    {
        println!("- checking if {file:?} exists in nym-data-observatory/migrations directory...");
        let filename = file
            .as_path()
            .file_name()
            .expect("migration filename is found");
        let filename = output_path.join(filename);
        println!(
            "- {} {file:?} => {filename:?} (exists = {})",
            if filename.exists() { "✅" } else { "❌" },
            filename.exists()
        );

        if !filename.exists() {
            anyhow::bail!(
                "migration {file:?} does not exist in nym-data-observatory/migrations directory, please check and copy it"
            );
        }
    }

    // sqlx
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        println!("cargo:rustc-env=DATABASE_URL={database_url}");
    }

    println!("✅ done");

    Ok(())
}
