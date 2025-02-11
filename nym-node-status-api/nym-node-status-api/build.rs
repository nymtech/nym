use anyhow::{anyhow, Result};
use sqlx::{Connection, SqliteConnection};
#[cfg(target_family = "unix")]
use std::fs::Permissions;
#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
use tokio::{fs::File, io::AsyncWriteExt};

const SQLITE_DB_FILENAME: &str = "nym-node-status-api.sqlite";

/// If you need to re-run migrations or reset the db, just run
/// cargo clean -p nym-node-status-api
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let out_dir = read_env_var("OUT_DIR")?;
    let database_path = format!("{}/{}?mode=rwc", out_dir, SQLITE_DB_FILENAME);

    write_db_path_to_file(&out_dir, SQLITE_DB_FILENAME).await?;
    let mut conn = SqliteConnection::connect(&database_path).await?;
    sqlx::migrate!("./migrations").run(&mut conn).await?;

    #[cfg(target_family = "unix")]
    println!("cargo::rustc-env=DATABASE_URL=sqlite://{}", &database_path);

    #[cfg(target_family = "windows")]
    // for some strange reason we need to add a leading `/` to the windows path even though it's
    // not a valid windows path... but hey, it works...
    println!("cargo::rustc-env=DATABASE_URL=sqlite:///{}", &database_path);

    Ok(())
}

fn read_env_var(var: &str) -> Result<String> {
    std::env::var(var).map_err(|_| anyhow!("You need to set {} env var", var))
}

/// use `./enter_db.sh` to inspect DB
async fn write_db_path_to_file(out_dir: &str, db_filename: &str) -> anyhow::Result<()> {
    let mut file = File::create("settings.sql").await?;
    let settings = ".mode columns
.headers on";
    file.write_all(settings.as_bytes()).await?;

    let mut file = File::create("enter_db.sh").await?;
    let contents = format!(
        "#!/bin/bash\n\
        sqlite3 -init settings.sql {}/{}",
        out_dir, db_filename,
    );
    file.write_all(contents.as_bytes()).await?;

    #[cfg(target_family = "unix")]
    file.set_permissions(Permissions::from_mode(0o755))
        .await
        .map_err(anyhow::Error::from)?;

    Ok(())
}
