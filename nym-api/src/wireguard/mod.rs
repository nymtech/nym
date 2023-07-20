// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WireguardKey {
    pub_key: [u8; 32],
    valid_until: DateTime<Utc>,
}

impl WireguardKey {
    pub fn new() -> Self {
        WireguardKey {
            pub_key: [0u8; 32],
            valid_until: DateTime::default(),
        }
    }
}
