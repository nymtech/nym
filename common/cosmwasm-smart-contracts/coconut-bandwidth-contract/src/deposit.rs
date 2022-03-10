// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositData {
    signing_public_key: String,
    encryption_public_key: String,
}

impl DepositData {
    pub fn new(signing_public_key: String, encryption_public_key: String) -> Self {
        DepositData {
            signing_public_key,
            encryption_public_key,
        }
    }
}
