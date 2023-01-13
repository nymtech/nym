// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[tokio::main]
async fn main() {
    #[cfg(feature = "fs-surb-storage")]
    {
        use sqlx::{Connection, SqliteConnection};
        use std::env;

        let out_dir = env::var("OUT_DIR").unwrap();
        let database_path = format!("{out_dir}/fs-surbs-example.sqlite");

        let mut conn = SqliteConnection::connect(&format!("sqlite://{database_path}?mode=rwc"))
            .await
            .expect("Failed to create SQLx database connection");

        sqlx::migrate!("./fs_surbs_migrations")
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
}
