// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use mixnet_contract_common::ContractStateParams;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractState {
    pub owner: Addr, // only the owner account can update state
    pub rewarding_validator_address: Addr,
    pub params: ContractStateParams,

    pub rewarding_in_progress: bool,
}
