// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;
use sqlx::migrate::Migrator;
use sqlx::{Connection, SqliteConnection};
use std::env;
use std::path::Path;

const MIGRATIONS_DIR: &str = "migrations";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Re-run this script whenever a migration is added or changed, so the throwaway database the
    // compile-time `query!` checks run against always has the current schema.
    println!("cargo:rerun-if-changed={MIGRATIONS_DIR}");

    let out_dir = env::var("OUT_DIR")?;
    let database_path = format!("{out_dir}/orchestrator.sqlite");

    // remove the db file if it already existed from previous build
    // in case it was from a different branch
    if std::fs::exists(&database_path)? {
        std::fs::remove_file(&database_path)?;
    }

    let mut conn = SqliteConnection::connect(&format!("sqlite://{database_path}?mode=rwc"))
        .await
        .context("Failed to create SQLx database connection")?;

    // Read the migrations at run time rather than with the `sqlx::migrate!` macro. The macro bakes
    // the migration set in at build-script COMPILE time, so a re-run triggered by the rerun-if-changed
    // above would still apply the old set and a brand-new migration would read as "no such table"
    // until this script was recompiled. Reading the directory at run time picks up new files on the
    // very next build.
    Migrator::new(Path::new(MIGRATIONS_DIR))
        .await
        .context("Failed to read SQLx migrations")?
        .run(&mut conn)
        .await
        .context("Failed to perform SQLx migrations")?;

    println!("cargo:rustc-env=DATABASE_URL=sqlite://{database_path}");

    Ok(())
}
