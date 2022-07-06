// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SpendCredentialData {
    funds: Coin,
    blinded_serial_number: String,
    gateway_cosmos_address: String,
}

impl SpendCredentialData {
    pub fn new(funds: Coin, blinded_serial_number: String, gateway_cosmos_address: String) -> Self {
        SpendCredentialData {
            funds,
            blinded_serial_number,
            gateway_cosmos_address,
        }
    }

    pub fn funds(&self) -> &Coin {
        &self.funds
    }

    pub fn blinded_serial_number(&self) -> &str {
        &self.blinded_serial_number
    }

    pub fn gateway_cosmos_address(&self) -> &str {
        &self.gateway_cosmos_address
    }
}
