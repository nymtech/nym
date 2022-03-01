// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::types::EncodedChannelPublicKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Specifies block height by which all issuers have to submit their secure channel keys
    /// so that they could participate in the NIDKG protocol.
    ///
    /// Note, it doesn't guarantee issuers would be able to start submitting their partial shares
    /// at that height unless at least predefined threshold of issuers have submitted their channel keys.
    pub initial_exchange_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // I guess it wouldn't be instantaneous? so that we could possibly batch add multiples?
    SubmitPublicKey { key: EncodedChannelPublicKey },
    LeaveIssuingSet,

    // would we be submitting PER issuer or for all of them at once?
    SubmitEncryptedShares { to_be_determined: () },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    IssuerDetails { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
