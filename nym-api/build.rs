use sqlx::{Connection, FromRow, SqliteConnection};
use std::env;

#[tokio::main]
async fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let database_path = format!("{}/nym-api-example.sqlite", out_dir);

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
