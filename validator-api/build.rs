use sqlx::{Connection, SqliteConnection};
use std::env;

#[tokio::main]
async fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let database_path = format!("sqlite://{}/validator-api-example.sqlite", out_dir);

    let mut conn = SqliteConnection::connect(&*format!("{}?mode=rwc", database_path))
        .await
        .expect("Failed to create SQLx database connection");

    sqlx::migrate!("./migrations")
        .run(&mut conn)
        .await
        .expect("Failed to perform SQLx migrations");

    println!("cargo:rustc-env=DATABASE_URL={}", &database_path);
}
