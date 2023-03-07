// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use vergen::{vergen, Config};

fn main() {
    let mut config = Config::default();
    if std::env::var("DOCS_RS").is_ok() {
        // If we don't have access to git information, such as in a docs.rs build, don't error
        *config.git_mut().skip_if_error_mut() = true;
    }
    vergen(Config::default()).expect("failed to extract build metadata")
}
