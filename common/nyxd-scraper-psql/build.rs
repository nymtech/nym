// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

fn main() {
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        println!("cargo::rustc-env=DATABASE_URL={database_url}");
    }
}
