use anyhow::{anyhow, Result};
use sqlx::{Connection, SqliteConnection};
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use tokio::{fs::File, io::AsyncWriteExt};

const SQLITE_DB_FILENAME: &str = "nym-node-status-api.sqlite";

/// If you need to re-run migrations or reset the db, just run
/// cargo clean -p nym-node-status-api
#[tokio::main]
async fn main() -> Result<()> {
    let out_dir = read_env_var("OUT_DIR")?;
    let database_path = format!("sqlite://{}/{}?mode=rwc", out_dir, SQLITE_DB_FILENAME);

    write_db_path_to_file(&out_dir, SQLITE_DB_FILENAME).await?;
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

/// use `./enter_db.sh` to inspect DB
async fn write_db_path_to_file(out_dir: &str, db_filename: &str) -> anyhow::Result<()> {
    let mut file = File::create("enter_db.sh").await?;
    let _ = file.write(b"#!/bin/bash\n").await?;
    file.write_all(format!("sqlite3 {}/{}", out_dir, db_filename).as_bytes())
        .await?;

    file.set_permissions(Permissions::from_mode(0o755))
        .await
        .map_err(From::from)
}
