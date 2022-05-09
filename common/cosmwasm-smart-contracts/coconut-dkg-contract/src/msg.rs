// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::{BlockHeight, EncodedBTEPublicKeyWithProof, EncodedEd25519PublicKey};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub dealing_exchange_beginning_height: BlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    RegisterDealer {
        ed25519_key: EncodedEd25519PublicKey,
        bte_key_with_proof: EncodedBTEPublicKeyWithProof,
        owner_signature: String,
    },
    CommitDealing {
        epoch_id: u32,
        dealing_digest: [u8; 32],
        // todo: or maybe list them explicitly, produce digest, etc?
        receivers: u32,
        // need to think if anything else is required
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetCurrentEpoch {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
