use sqlx::{Connection, FromRow, SqliteConnection};
use std::{env, fs::Permissions, os::unix::fs::PermissionsExt};
use tokio::{fs::File, io::AsyncWriteExt};

const SQLITE_DB_FILENAME: &str = "nym-api-example.sqlite";

// it's fine if compilation fails
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[tokio::main]
async fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let database_path = format!("{}/{}", out_dir, SQLITE_DB_FILENAME);

    write_db_path_to_file(&out_dir, SQLITE_DB_FILENAME)
        .await
        .expect("Failed to write DB path to file");

    let mut conn = SqliteConnection::connect(&format!("sqlite://{}?mode=rwc", database_path))
        .await
        .expect("Failed to create SQLx database connection");

    sqlx::migrate!("./migrations")
        .run(&mut conn)
        .await
        .expect("Failed to perform SQLx migrations");

    #[derive(FromRow)]
    struct Exists {
        exists: bool,
    }

    // check if it was already run
    let res: Exists = sqlx::query_as("SELECT EXISTS (SELECT 1 FROM v3_migration_info) AS 'exists'")
        .fetch_one(&mut conn)
        .await
        .unwrap();

    let already_run = res.exists;

    // execute the manual v3 migration
    // it's performed on an empty storage, so we don't need to actually make any network queries
    if !already_run {
        sqlx::query(
            r#"
                CREATE TABLE gateway_details_temp
                (
                    id       INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                    node_id  INTEGER NOT NULL UNIQUE,
                    identity VARCHAR NOT NULL UNIQUE
                );

                DROP TABLE gateway_details;
                ALTER TABLE gateway_details_temp RENAME TO gateway_details;

                INSERT INTO v3_migration_info(id) VALUES (0);
            "#,
        )
        .execute(&mut conn)
        .await
        .expect("failed to update post v3 migration tables");
    }

    #[cfg(target_family = "unix")]
    println!("cargo:rustc-env=DATABASE_URL=sqlite://{}", &database_path);

    #[cfg(target_family = "windows")]
    // for some strange reason we need to add a leading `/` to the windows path even though it's
    // not a valid windows path... but hey, it works...
    println!("cargo:rustc-env=DATABASE_URL=sqlite:///{}", &database_path);
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
