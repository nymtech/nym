use std::collections::HashMap;
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

    // hash contents of files in common migrations
    let mut common_migrations_hashes = HashMap::new();
    for file in glob(&format!("{}/*", common_migrations_path.to_str().unwrap()))
        .unwrap()
        .flatten()
    {
        let hash = blake3::hash(std::fs::read(&file)?.as_slice());
        common_migrations_hashes.insert(hash, file);
    }

    // hash contents of files in data observatory migrations
    let mut data_observatory_migrations_hashes = HashMap::new();
    for file in glob(&format!("{}/*", output_path.to_str().unwrap()))
        .unwrap()
        .flatten()
    {
        let hash = blake3::hash(std::fs::read(&file)?.as_slice());
        data_observatory_migrations_hashes.insert(hash, file);
    }

    let mut errors = vec![];

    for entry in common_migrations_hashes {
        println!(
            "- checking if {:?} exists in nym-data-observatory/migrations directory...",
            entry.1
        );
        let res = data_observatory_migrations_hashes.get(&entry.0);
        let res_path = res.and_then(|r| r.to_str()).unwrap_or("(not found)");
        println!(
            "- {} {} => {res_path} (content matches = {})",
            if res.is_some() { "✅" } else { "❌" },
            entry.1.as_path().to_str().unwrap(),
            res.is_some()
        );

        if res.is_none() {
            errors.push(format!("- {:?}", entry.1.as_path()));
        }
    }

    // show all errors
    if !errors.is_empty() {
        anyhow::bail!(
            "the following migrations have changed or do not exist in nym-data-observatory/migrations directory, please check and copy them:\n{}",
            errors.join("\n")
        );
    }

    // sqlx
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        println!("cargo:rustc-env=DATABASE_URL={database_url}");
    }

    println!("✅ done");

    Ok(())
}
