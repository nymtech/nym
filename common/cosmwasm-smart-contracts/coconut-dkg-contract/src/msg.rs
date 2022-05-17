// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{BlockHeight, EncodedBTEPublicKeyWithProof, EncodedEd25519PublicKey};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub public_key_submission_end_height: BlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterDealer {
        ed25519_key: EncodedEd25519PublicKey,
        bte_key_with_proof: EncodedBTEPublicKeyWithProof,
        owner_signature: String,
        host: String,
    },
    CommitDealing {
        epoch_id: u32,
        dealing_digest: [u8; 32],
        // todo: or maybe list them explicitly, produce digest, etc?
        receivers: u32,
        // need to think if anything else is required
    },

    // only exists for debugging purposes on local network to reset the entire state of the contract
    UnsafeResetAll {
        init_msg: InstantiateMsg,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetCurrentEpoch {},
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
    GetBlacklistedDealers {
        limit: Option<u32>,
        start_after: Option<String>,
    },
    GetBlacklisting {
        dealer: String,
    },
    GetDepositAmount {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
