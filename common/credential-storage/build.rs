/*
 * Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

use sqlx::{Connection, SqliteConnection};
use std::env;

#[tokio::main]
async fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let database_path = format!("{}/coconut-credential-example.sqlite", out_dir);

    let mut conn = SqliteConnection::connect(&*format!("sqlite://{}?mode=rwc", database_path))
        .await
        .expect("Failed to create SQLx database connection");

    sqlx::migrate!("./migrations")
        .run(&mut conn)
        .await
        .expect("Failed to perform SQLx migrations");

    #[cfg(target_family = "unix")]
    println!("cargo:rustc-env=DATABASE_URL=sqlite://{}", &database_path);

    #[cfg(target_family = "windows")]
    // for some strange reason we need to add a leading `/` to the windows path even though it's
    // not a valid windows path... but hey, it works...
    println!("cargo:rustc-env=DATABASE_URL=sqlite:///{}", &database_path);
}
