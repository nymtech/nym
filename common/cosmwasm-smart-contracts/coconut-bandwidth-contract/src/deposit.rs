// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositData {
    verification_key: String,
    encryption_key: String,
}

impl DepositData {
    pub fn new(verification_key: String, encryption_key: String) -> Self {
        DepositData {
            verification_key,
            encryption_key,
        }
    }
}
