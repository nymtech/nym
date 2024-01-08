// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_binary, to_binary, Addr, Coin, CosmosMsg, StdResult, WasmMsg};
use nym_multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;

use crate::msg::ExecuteMsg;

#[cw_serde]
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

#[cw_serde]
#[derive(Copy)]
pub enum SpendCredentialStatus {
    #[serde(alias = "InProgress")]
    InProgress,
    #[serde(alias = "Spent")]
    Spent,
}

#[cw_serde]
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

#[cw_serde]
pub struct PagedSpendCredentialResponse {
    pub spend_credentials: Vec<SpendCredential>,
    pub per_page: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
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

#[cw_serde]
pub struct SpendCredentialResponse {
    pub spend_credential: Option<SpendCredential>,
}

impl SpendCredentialResponse {
    pub fn new(spend_credential: Option<SpendCredential>) -> Self {
        SpendCredentialResponse { spend_credential }
    }
}

pub fn to_cosmos_msg(
    funds: Coin,
    blinded_serial_number: String,
    coconut_bandwidth_addr: String,
    multisig_addr: String,
) -> StdResult<CosmosMsg> {
    let release_funds_req = ExecuteMsg::ReleaseFunds { funds };
    let release_funds_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: coconut_bandwidth_addr,
        msg: to_binary(&release_funds_req)?,
        funds: vec![],
    });
    let req = MultisigExecuteMsg::Propose {
        title: String::from("Release funds, as ordered by Coconut Bandwidth Contract"),
        description: blinded_serial_number,
        msgs: vec![release_funds_msg],
        latest: None,
    };
    let msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: multisig_addr,
        msg: to_binary(&req)?,
        funds: vec![],
    });

    Ok(msg)
}

pub fn funds_from_cosmos_msgs(msgs: Vec<CosmosMsg>) -> Option<Coin> {
    if let Some(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: _,
        msg,
        funds: _,
    })) = msgs.first()
    {
        if let Ok(ExecuteMsg::ReleaseFunds { funds }) = from_binary::<ExecuteMsg>(msg) {
            return Some(funds);
        }
    }
    None
}
