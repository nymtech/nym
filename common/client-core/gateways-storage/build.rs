// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "fs-gateways-storage")]
    {
        use anyhow::Context;
        use sqlx::migrate::Migrator;
        use sqlx::{Connection, SqliteConnection};
        use std::env;
        use std::path::Path;

        const MIGRATIONS_DIR: &str = "fs_gateways_migrations";

        // re-run whenever a migration is added or changed, so the `query!` checks see the current schema
        println!("cargo:rerun-if-changed={MIGRATIONS_DIR}");

        let out_dir = env::var("OUT_DIR")?;
        let database_path = format!("{out_dir}/gateways-storage-example.sqlite");

        // remove the db file if it already existed from previous build
        // in case it was from a different branch
        if std::fs::exists(&database_path)? {
            std::fs::remove_file(&database_path)?;
        }

        let mut conn = SqliteConnection::connect(&format!("sqlite://{database_path}?mode=rwc"))
            .await
            .context("Failed to create SQLx database connection")?;

        // read at run time: `sqlx::migrate!` would bake in the set from when this script was compiled
        Migrator::new(Path::new(MIGRATIONS_DIR))
            .await
            .context("Failed to read SQLx migrations")?
            .run(&mut conn)
            .await
            .context("Failed to perform SQLx migrations")?;

        println!("cargo:rustc-env=DATABASE_URL=sqlite://{database_path}");
    }

    Ok(())
}
