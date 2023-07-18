// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::families::{Family, FamilyHead};
use crate::{Layer, RewardedSetNodeStatus};
use contracts_common::IdentityKey;
use cosmwasm_std::Addr;
use cosmwasm_std::Coin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Index;

// type aliases for better reasoning about available data
pub type SphinxKey = String;
pub type SphinxKeyRef<'a> = &'a str;
pub type EpochId = u32;
pub type IntervalId = u32;
pub type MixId = u32;
pub type BlockHeight = u64;
pub type EpochEventId = u32;
pub type IntervalEventId = u32;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, PartialEq, Eq)]
pub struct LayerAssignment {
    mix_id: MixId,
    layer: Layer,
}

impl LayerAssignment {
    pub fn new(mix_id: MixId, layer: Layer) -> Self {
        LayerAssignment { mix_id, layer }
    }

    pub fn mix_id(&self) -> MixId {
        self.mix_id
    }

    pub fn layer(&self) -> Layer {
        self.layer
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Copy, Clone, Eq, PartialEq)]
pub struct LayerDistribution {
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

        // we explicitly put 3 elements into the iterator, so the iterator is DEFINITELY
        // not empty and thus the unwrap cannot fail
        #[allow(clippy::unwrap_used)]
        layers.iter().min_by_key(|x| x.1).unwrap().0
    }

    pub fn increment_layer_count(&mut self, layer: Layer) {
        match layer {
            Layer::One => self.layer1 += 1,
            Layer::Two => self.layer2 += 1,
            Layer::Three => self.layer3 += 1,
        }
    }

    pub fn decrement_layer_count(&mut self, layer: Layer) -> Result<(), MixnetContractError> {
        match layer {
            Layer::One => {
                self.layer1 =
                    self.layer1
                        .checked_sub(1)
                        .ok_or(MixnetContractError::OverflowSubtraction {
                            minuend: self.layer1,
                            subtrahend: 1,
                        })?
            }
            Layer::Two => {
                self.layer2 =
                    self.layer2
                        .checked_sub(1)
                        .ok_or(MixnetContractError::OverflowSubtraction {
                            minuend: self.layer2,
                            subtrahend: 1,
                        })?
            }
            Layer::Three => {
                self.layer3 =
                    self.layer3
                        .checked_sub(1)
                        .ok_or(MixnetContractError::OverflowSubtraction {
                            minuend: self.layer3,
                            subtrahend: 1,
                        })?
            }
        }

        Ok(())
    }
}

impl Index<Layer> for LayerDistribution {
    type Output = u64;

    fn index(&self, index: Layer) -> &Self::Output {
        match index {
            Layer::One => &self.layer1,
            Layer::Two => &self.layer2,
            Layer::Three => &self.layer3,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractState {
    pub owner: Addr, // only the owner account can update state
    pub rewarding_validator_address: Addr,

    /// Address of the vesting contract to which the mixnet contract would be sending all
    /// track-related messages.
    pub vesting_contract_address: Addr,
    pub rewarding_denom: String,
    pub params: ContractStateParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractStateParams {
    /// Minimum amount a delegator must stake in orders for his delegation to get accepted.
    pub minimum_mixnode_delegation: Option<Coin>,

    /// Minimum amount a mixnode must pledge to get into the system.
    pub minimum_mixnode_pledge: Coin,

    /// Minimum amount a gateway must pledge to get into the system.
    pub minimum_gateway_pledge: Coin,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
pub struct PagedRewardedSetResponse {
    pub nodes: Vec<(MixId, RewardedSetNodeStatus)>,
    pub start_next_after: Option<MixId>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
pub struct PagedFamiliesResponse {
    pub families: Vec<Family>,
    pub start_next_after: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
pub struct PagedMembersResponse {
    pub members: Vec<(IdentityKey, FamilyHead)>,
    pub start_next_after: Option<String>,
}
