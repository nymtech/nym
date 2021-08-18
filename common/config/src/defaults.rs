// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// re-export everything from the `network-defaults` to not break the existing imports
// reason for moving defaults to separate crate is that I don't want to pull in all dependencies
// like `handlebars`, `toml`, etc if I only want to grab one constant...
pub use network_defaults::*;
