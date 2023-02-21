// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ContractSafeBytes, EncodedBTEPublicKeyWithProof, TimeConfiguration};
use crate::verification_key::VerificationKeyShare;
use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub group_addr: String,
    pub multisig_addr: String,
    pub time_configuration: Option<TimeConfiguration>,
    pub mix_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterDealer {
        bte_key_with_proof: EncodedBTEPublicKeyWithProof,
        announce_address: String,
    },

    CommitDealing {
        dealing_bytes: ContractSafeBytes,
    },

    CommitVerificationKeyShare {
        share: VerificationKeyShare,
    },

    VerifyVerificationKeyShare {
        owner: Addr,
    },

    AdvanceEpochState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetCurrentEpochState {},
    GetCurrentEpochThreshold {},
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
    GetDealing {
        idx: u64,
        limit: Option<u32>,
        start_after: Option<String>,
    },
    GetVerificationKeys {
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
