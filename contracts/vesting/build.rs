// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use vergen::{vergen, Config};

fn main() {
    #[cfg(not(target_family = "windows"))]
    vergen(Config::default()).expect("failed to extract build metadata")
}
