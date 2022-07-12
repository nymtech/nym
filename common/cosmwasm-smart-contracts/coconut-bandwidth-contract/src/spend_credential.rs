// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{from_binary, to_binary, Addr, Coin, CosmosMsg, StdResult, WasmMsg};
use multisig_contract_common::msg::ExecuteMsg as MultisigExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::ExecuteMsg;

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

    pub fn to_cosmos_msg(
        &self,
        coconut_bandwidth_addr: String,
        multisig_addr: String,
    ) -> StdResult<CosmosMsg> {
        let release_funds_req = ExecuteMsg::ReleaseFunds {
            funds: self.funds.clone(),
        };
        let release_funds_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: coconut_bandwidth_addr,
            msg: to_binary(&release_funds_req)?,
            funds: vec![],
        });
        let req = MultisigExecuteMsg::Propose {
            title: String::from("Release funds, as ordered by Coconut Bandwidth Contract"),
            description: self.blinded_serial_number.clone(),
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
        })) = msgs.get(0)
        {
            if let Ok(MultisigExecuteMsg::Propose {
                title: _,
                description: _,
                msgs,
                latest: _,
            }) = from_binary(&msg)
            {
                if let Some(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: _,
                    msg,
                    funds: _,
                })) = msgs.get(0)
                {
                    if let Ok(ExecuteMsg::ReleaseFunds { funds }) = from_binary(&msg) {
                        return Some(funds);
                    }
                }
            }
        }
        None
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct SpendCredentialResponse {
    pub spend_credential: Option<SpendCredential>,
}

impl SpendCredentialResponse {
    pub fn new(spend_credential: Option<SpendCredential>) -> Self {
        SpendCredentialResponse { spend_credential }
    }
}
