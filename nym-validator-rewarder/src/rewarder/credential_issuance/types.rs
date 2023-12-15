// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_validator_client::nyxd::{AccountId, Coin};

pub struct CredentialIssuanceResults {
    pub api_runners: Vec<()>,
}

impl CredentialIssuanceResults {
    pub fn rewarding_amounts(&self, budget: &Coin) -> Vec<(AccountId, Vec<Coin>)> {
        let _ = budget;
        Vec::new()
    }
}
