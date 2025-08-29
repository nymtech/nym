// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use sqlx::{Connection, SqliteConnection};
    use std::env;

    let out_dir = env::var("OUT_DIR")?;
    let database_path = format!("{out_dir}/scraper-example.sqlite");

    let mut conn = SqliteConnection::connect(&format!("sqlite://{database_path}?mode=rwc"))
        .await
        .context("Failed to create SQLx database connection")?;

    sqlx::migrate!("./migrations")
        .run(&mut conn)
        .await
        .context("Failed to perform SQLx migrations")?;

    #[cfg(target_family = "unix")]
    println!("cargo:rustc-env=DATABASE_URL=sqlite://{}", &database_path);

    #[cfg(target_family = "windows")]
    // for some strange reason we need to add a leading `/` to the windows path even though it's
    // not a valid windows path... but hey, it works...
    println!("cargo:rustc-env=DATABASE_URL=sqlite:///{}", &database_path);

    Ok(())
}
