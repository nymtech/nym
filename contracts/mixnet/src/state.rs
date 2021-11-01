// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Decimal};
use mixnet_contract::StateParams;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr, // only the owner account can update state
    pub network_monitor_address: Addr,
    pub params: StateParams,

    // helper values to avoid having to recalculate them on every single payment operation
    pub mixnode_epoch_bond_reward: Decimal, // reward per epoch expressed as a decimal like 0.05
    pub mixnode_epoch_delegation_reward: Decimal, // reward per epoch expressed as a decimal like 0.05
}
