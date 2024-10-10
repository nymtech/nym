// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[tokio::main]
async fn main() {
    use sqlx::{Connection, SqliteConnection};
    use std::env;

    let out_dir = env::var("OUT_DIR").unwrap();
    let database_path = format!("{out_dir}/nym-vpn-api-example.sqlite");

    let mut conn = SqliteConnection::connect(&format!("sqlite://{database_path}?mode=rwc"))
        .await
        .expect("Failed to create SQLx database connection");

    sqlx::migrate!("./migrations")
        .run(&mut conn)
        .await
        .expect("Failed to perform SQLx migrations");

    println!("cargo:rustc-env=DATABASE_URL=sqlite://{}", &database_path);
}
