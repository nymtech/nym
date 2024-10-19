use anyhow::Result;
use sqlx::{Connection, PgConnection};
use std::io::Write;
use std::{collections::HashMap, fs::File};

const POSTGRES_USER: &str = "nym";
const POSTGRES_PASSWORD: &str = "password123";
const POSTGRES_DB: &str = "data_obs_db";

/// if schema changes, rerun `cargo sqlx prepare` with a running DB
/// https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query
#[tokio::main]
async fn main() -> Result<()> {
    let db_url =
        format!("postgresql://{POSTGRES_USER}:{POSTGRES_PASSWORD}@localhost:5432/{POSTGRES_DB}");

    export_db_variables(&db_url)?;
    // if a live DB is reachable, use that
    if PgConnection::connect(&db_url).await.is_ok() {
        println!("cargo::rustc-env=SQLX_OFFLINE=false");
        run_migrations(&db_url).await?;
    } else {
        // by default, run in offline mode
        println!("cargo::rustc-env=SQLX_OFFLINE=true");
    }

    rerun_if_changed();

    Ok(())
}

fn export_db_variables(db_url: &str) -> Result<()> {
    let mut map = HashMap::new();
    map.insert("POSTGRES_USER", POSTGRES_USER);
    map.insert("POSTGRES_PASSWORD", POSTGRES_PASSWORD);
    map.insert("POSTGRES_DB", POSTGRES_DB);
    map.insert("DATABASE_URL", db_url);

    let mut file = File::create(".env")?;
    for (var, value) in map.iter() {
        println!("cargo::rustc-env={}={}", var, value);
        writeln!(file, "{}={}", var, value).expect("Failed to write to dotenv file");
    }

    Ok(())
}

async fn run_migrations(db_url: &str) -> Result<()> {
    let mut conn = PgConnection::connect(db_url).await?;
    sqlx::migrate!("./migrations").run(&mut conn).await?;

    Ok(())
}

fn rerun_if_changed() {
    println!("cargo::rerun-if-changed=migrations");
    println!("cargo::rerun-if-changed=src/db/queries");
}
