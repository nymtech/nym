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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateParams {
    pub epoch_length: u32, // length of an epoch, expressed in hours

    pub minimum_mixnode_bond: Uint128, // minimum amount a mixnode must bond to get into the system
    pub minimum_gateway_bond: Uint128, // minimum amount a gateway must bond to get into the system
    pub mixnode_bond_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25
    pub gateway_bond_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25
    pub mixnode_delegation_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25
    pub gateway_delegation_reward_rate: Decimal, // annual reward rate, expressed as a decimal like 1.25
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
            "gateway bond reward rate: {}; ",
            self.gateway_bond_reward_rate
        )?;
        write!(
            f,
            "mixnode delegation reward rate: {}; ",
            self.mixnode_delegation_reward_rate
        )?;
        write!(
            f,
            "gateway delegation reward rate: {}; ",
            self.gateway_delegation_reward_rate
        )?;
        write!(
            f,
            "mixnode active set size: {} ]",
            self.mixnode_active_set_size
        )
    }
}

// type aliases for better reasoning about available data
pub type IdentityKey = String;
pub type IdentityKeyRef<'a> = &'a str;
pub type SphinxKey = String;
