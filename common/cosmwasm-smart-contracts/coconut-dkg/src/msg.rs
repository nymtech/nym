// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ContractSafeBytes, EncodedBTEPublicKeyWithProof};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub group_addr: String,
    pub mix_denom: String,
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterDealer {
        bte_key_with_proof: EncodedBTEPublicKeyWithProof,
    },

    CommitDealing {
        dealing_bytes: ContractSafeBytes,
    },

    AdvanceEpochState {},

    // DEBUG ONLY TXs. THEY SHALL BE REMOVED BEFORE FINALISING THE CODE
    // only exists for debugging purposes on local network to reset the entire state of the contract
    DebugUnsafeResetAll {
        init_msg: InstantiateMsg,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetCurrentEpochState {},
    GetDealerDetails {
        dealer_address: String,
    },
    GetCurrentDealers {
        limit: Option<u32>,
        start_after: Option<String>,
    },
    GetPastDealers {
        limit: Option<u32>,
        start_after: Option<String>,
    },
    GetDepositAmount {},
    GetDealing {
        idx: u64,
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
