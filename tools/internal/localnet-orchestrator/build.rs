use anyhow::Context;
use sqlx::{Connection, SqliteConnection};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let out_dir = env::var("OUT_DIR")?;
    let database_path = format!("{out_dir}/localnet-example.sqlite");

    // remove the db file if it already existed from previous build
    // in case it was from a different branch
    if std::fs::exists(&database_path)? {
        std::fs::remove_file(&database_path)?;
    }

    let mut conn = SqliteConnection::connect(&format!("sqlite://{database_path}?mode=rwc"))
        .await
        .context("Failed to create SQLx database connection")?;

    sqlx::migrate!("./migrations")
        .run(&mut conn)
        .await
        .context("Failed to perform SQLx migrations")?;

    println!("cargo:rustc-env=DATABASE_URL=sqlite://{}", &database_path);

    Ok(())
}
