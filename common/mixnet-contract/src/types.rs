// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::Layer;
use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub struct LayerDistribution {
    pub gateways: u64,
    pub layer1: u64,
    pub layer2: u64,
    pub layer3: u64,
}

impl LayerDistribution {
    pub fn choose_with_fewest(&self) -> Layer {
        let layers = [
            (Layer::One, self.layer1),
            (Layer::Two, self.layer2),
            (Layer::Three, self.layer3),
        ];
        layers.iter().min_by_key(|x| x.1).unwrap().0
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub struct RewardingIntervalResponse {
    pub current_rewarding_interval_starting_block: u64,
    pub current_rewarding_interval_nonce: u32,
    pub rewarding_in_progress: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateParams {
    pub epoch_length: u32, // length of a rewarding epoch/interval, expressed in hours

    pub minimum_mixnode_bond: Uint128, // minimum amount a mixnode must bond to get into the system
    pub minimum_gateway_bond: Uint128, // minimum amount a gateway must bond to get into the system

    pub mixnode_bond_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25
    pub mixnode_delegation_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25

    // number of mixnode that are going to get rewarded during current rewarding interval (k_m)
    // based on overall demand for private bandwidth-
    pub mixnode_rewarded_set_size: u32,

    // subset of rewarded mixnodes that are actively receiving mix traffic
    // used to handle shorter-term (e.g. hourly) fluctuations of demand
    pub mixnode_active_set_size: u32,
}

impl Display for StateParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Contract state parameters: [ ")?;
        write!(f, "epoch length: {}; ", self.epoch_length)?;
        write!(f, "minimum mixnode bond: {}; ", self.minimum_mixnode_bond)?;
        write!(f, "minimum gateway bond: {}; ", self.minimum_gateway_bond)?;
        write!(
            f,
            "mixnode bond reward rate: {}; ",
            self.mixnode_bond_reward_rate
        )?;
        write!(
            f,
            "mixnode delegation reward rate: {}; ",
            self.mixnode_delegation_reward_rate
        )?;
        write!(
            f,
            "mixnode rewarded set size: {}",
            self.mixnode_rewarded_set_size
        )?;
        write!(
            f,
            "mixnode active set size: {}",
            self.mixnode_active_set_size
        )
    }
}

// type aliases for better reasoning about available data
pub type IdentityKey = String;
pub type IdentityKeyRef<'a> = &'a str;
pub type SphinxKey = String;
