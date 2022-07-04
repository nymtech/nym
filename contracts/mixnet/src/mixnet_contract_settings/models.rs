// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use mixnet_contract_common::ContractStateParams;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractState {
    pub owner: Addr, // only the owner account can update state
    pub mix_denom: String,
    pub rewarding_validator_address: Addr,
    pub rewarding_denom: String,
    pub params: ContractStateParams,
}
