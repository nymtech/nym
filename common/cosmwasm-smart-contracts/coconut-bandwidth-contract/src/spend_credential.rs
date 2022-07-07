// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Coin};
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub enum SpendCredentialStatus {
    InProgress,
    Spent,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct SpendCredential {
    funds: Coin,
    blinded_serial_number: String,
    gateway_cosmos_address: Addr,
    status: SpendCredentialStatus,
}

impl SpendCredential {
    pub fn new(funds: Coin, blinded_serial_number: String, gateway_cosmos_address: Addr) -> Self {
        SpendCredential {
            funds,
            blinded_serial_number,
            gateway_cosmos_address,
            status: SpendCredentialStatus::InProgress,
        }
    }

    pub fn blinded_serial_number(&self) -> &str {
        &self.blinded_serial_number
    }

    pub fn status(&self) -> SpendCredentialStatus {
        self.status
    }

    pub fn mark_as_spent(&mut self) {
        self.status = SpendCredentialStatus::Spent;
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedSpendCredentialResponse {
    pub spend_credentials: Vec<SpendCredential>,
    pub per_page: usize,
    pub start_next_after: Option<String>,
}

impl PagedSpendCredentialResponse {
    pub fn new(
        spend_credentials: Vec<SpendCredential>,
        per_page: usize,
        start_next_after: Option<String>,
    ) -> Self {
        PagedSpendCredentialResponse {
            spend_credentials,
            per_page,
            start_next_after,
        }
    }
}
