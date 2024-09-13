use anyhow::{anyhow, Result};
use sqlx::{Connection, SqliteConnection};

const SQLITE_DB_FILENAME: &str = "nym-node-status-api.sqlite";

#[tokio::main]
async fn main() -> Result<()> {
    let out_dir = read_env_var("OUT_DIR")?;
    let database_path = format!("sqlite://{}/{}?mode=rwc", out_dir, SQLITE_DB_FILENAME);

    let mut conn = SqliteConnection::connect(&database_path).await?;
    sqlx::migrate!("./migrations").run(&mut conn).await?;

    #[cfg(target_family = "unix")]
    println!("cargo::rustc-env=DATABASE_URL=sqlite://{}", &database_path);

    #[cfg(target_family = "windows")]
    // for some strange reason we need to add a leading `/` to the windows path even though it's
    // not a valid windows path... but hey, it works...
    println!("cargo::rustc-env=DATABASE_URL=sqlite:///{}", &database_path);

    rerun_if_changed();
    Ok(())
}

fn read_env_var(var: &str) -> Result<String> {
    std::env::var(var).map_err(|_| anyhow!("You need to set {} env var", var))
}

fn rerun_if_changed() {
    println!("cargo::rerun-if-changed=migrations");
    println!("cargo::rerun-if-changed=src/db/queries");
}
