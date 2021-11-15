// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Decimal};
use mixnet_contract::StateParams;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr, // only the owner account can update state
    pub rewarding_validator_address: Addr,
    pub params: StateParams,

    // keep track of the changes to the current rewarding interval,
    // i.e. at which block has the latest rewarding occurred
    // and whether another run is already in progress
    pub rewarding_interval_starting_block: u64,
    pub latest_rewarding_interval_nonce: u32,
    pub rewarding_in_progress: bool,
}
