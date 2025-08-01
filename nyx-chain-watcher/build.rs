use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, Connection, SqliteConnection};
use std::env::var;
use std::io::Write;
use std::{collections::HashMap, fs::File, path::PathBuf, str::FromStr};

#[tokio::main]
async fn main() -> Result<()> {
    let db_path = PathBuf::from(var("OUT_DIR").unwrap()).join("nyx_chain_watcher.sqlite");

    // Create the database directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_path_str = db_path.display().to_string().replace('\\', "/");
    let db_url = format!("sqlite:{db_path_str}");

    // Ensure database file is created with proper permissions
    let connect_options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    // Create initial connection to ensure database exists
    let mut conn = SqliteConnection::connect_with(&connect_options).await?;

    sqlx::migrate!("./migrations").run(&mut conn).await?;
    export_db_variables(&db_url)?;

    // Force SQLx to prepare all queries during build
    println!("cargo:rustc-env=SQLX_OFFLINE=true");
    println!("cargo:rustc-env=DATABASE_URL={db_url}");

    // Add rerun-if-changed directives
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src");

    Ok(())
}

fn export_db_variables(db_url: &str) -> Result<()> {
    let mut map = HashMap::new();
    map.insert("DATABASE_URL", db_url);

    let mut file = File::create(".env")?;
    for (var, value) in map.iter() {
        writeln!(file, "{var}={value}")?;
    }

    Ok(())
}
