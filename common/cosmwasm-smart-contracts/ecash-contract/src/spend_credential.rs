// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::msg::ExecuteMsg;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_binary, CosmosMsg, WasmMsg};

#[cw_serde]
pub struct EcashSpentCredential {
    serial_number: String,
    gateway_cosmos_address: String,
}

impl EcashSpentCredential {
    pub fn new(serial_number: String, gateway_cosmos_address: String) -> Self {
        EcashSpentCredential {
            serial_number,
            gateway_cosmos_address,
        }
    }

    pub fn serial_number(&self) -> &str {
        &self.serial_number
    }
}

#[cw_serde]
pub struct PagedEcashSpentCredentialResponse {
    pub spend_credentials: Vec<EcashSpentCredential>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<String>,
}

impl PagedEcashSpentCredentialResponse {
    pub fn new(
        spend_credentials: Vec<EcashSpentCredential>,
        per_page: usize,
        start_next_after: Option<String>,
    ) -> Self {
        PagedEcashSpentCredentialResponse {
            spend_credentials,
            per_page,
            start_next_after,
        }
    }
}

#[cw_serde]
pub struct EcashSpentCredentialResponse {
    pub spend_credential: Option<EcashSpentCredential>,
}

impl EcashSpentCredentialResponse {
    pub fn new(spend_credential: Option<EcashSpentCredential>) -> Self {
        EcashSpentCredentialResponse { spend_credential }
    }
}

pub fn check_proposal(msgs: Vec<CosmosMsg>) -> bool {
    if let Some(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: _,
        msg,
        funds: _,
    })) = msgs.first()
    {
        if let Ok(ExecuteMsg::SpendCredential {
            serial_number: _,
            gateway_cosmos_address: _,
        }) = from_binary::<ExecuteMsg>(msg)
        {
            return true;
        }
    }
    false
}
