// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(
    PartialEq,
    Copy,
    Clone,
    strum_macros::Display,
    strum_macros::EnumString,
    Serialize,
    Deserialize,
    Default,
    Debug,
)]
pub enum SessionType {
    Vpn,
    Mixnet,
    Wasm,
    Native,
    Socks5,
    #[default]
    Unknown,
}

impl SessionType {
    pub fn from_string<S: AsRef<str>>(s: S) -> Self {
        s.as_ref().parse().unwrap_or_default()
    }
}
