use anyhow::Result;
use sqlx::{Connection, PgConnection};
use std::io::Write;
use std::{collections::HashMap, fs::File};

const POSTGRES_USER: &str = "nym";
const POSTGRES_PASSWORD: &str = "password123";
const POSTGRES_DB: &str = "data_obs_db";

/// If you need to re-run migrations or reset the db, just run
/// cargo clean -p nym-node-status-api
#[tokio::main]
async fn main() -> Result<()> {
    if let Ok(value) = std::env::var("CI") {
        if value == "true" {
            println!("cargo::rustc-env=SQLX_OFFLINE=true");
        }
    } else {
        let db_url = export_db_variables()?;
        run_migrations(&db_url).await?;
    }

    rerun_if_changed();
    Ok(())
}

fn export_db_variables() -> Result<String> {
    let mut map = HashMap::new();
    map.insert("POSTGRES_USER", POSTGRES_USER);
    map.insert("POSTGRES_PASSWORD", POSTGRES_PASSWORD);
    map.insert("POSTGRES_DB", POSTGRES_DB);
    let db_url = format!(
        "postgresql://{}:{}@localhost:5432/{}",
        POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
    );
    map.insert("DATABASE_URL", db_url.as_str());

    let mut file = File::create(".env")?;
    for (var, value) in map.iter() {
        println!("cargo::rustc-env={}={}", var, value);
        writeln!(file, "{}={}", var, value).expect("Failed to write to dotenv file");
    }

    Ok(db_url)
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
