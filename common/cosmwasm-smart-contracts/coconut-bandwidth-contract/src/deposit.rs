// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DepositData {
    deposit_info: String,
    identity_key: String,
    encryption_key: String,
}

impl DepositData {
    pub fn new(deposit_info: String, identity_key: String, encryption_key: String) -> Self {
        DepositData {
            deposit_info,
            identity_key,
            encryption_key,
        }
    }

    pub fn deposit_info(&self) -> &str {
        &self.deposit_info
    }

    pub fn identity_key(&self) -> &str {
        &self.identity_key
    }

    pub fn encryption_key(&self) -> &str {
        &self.encryption_key
    }
}
