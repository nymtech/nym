// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::Layer;
use contracts_common::Percent;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use cosmwasm_std::{Addr, Uint128};
use std::fmt::{Display, Formatter};
use std::ops::Index;

// type aliases for better reasoning about available data
pub type SphinxKey = String;
pub type SphinxKeyRef<'a> = &'a str;

pub type MixId = u32;
pub type BlockHeight = u64;

#[cw_serde]
pub struct RangedValue<T> {
    pub minimum: T,
    pub maximum: T,
}

impl<T> Copy for RangedValue<T> where T: Copy {}

pub type ProfitMarginRange = RangedValue<Percent>;
pub type OperatingCostRange = RangedValue<Uint128>;

impl<T> Display for RangedValue<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.minimum, self.maximum)
    }
}

impl Default for ProfitMarginRange {
    fn default() -> Self {
        ProfitMarginRange {
            minimum: Percent::zero(),
            maximum: Percent::hundred(),
        }
    }
}

impl Default for OperatingCostRange {
    fn default() -> Self {
        OperatingCostRange {
            minimum: Uint128::zero(),

            // 1 billion (native tokens, i.e. 1 billion * 1'000'000 base tokens) - the total supply
            maximum: Uint128::new(1_000_000_000_000_000),
        }
    }
}

impl<T> RangedValue<T>
where
    T: Copy + PartialOrd + PartialEq,
{
    pub fn normalise(&self, value: T) -> T {
        if value < self.minimum {
            self.minimum
        } else if value > self.maximum {
            self.maximum
        } else {
            value
        }
    }

    pub fn within_range(&self, value: T) -> bool {
        value >= self.minimum && value <= self.maximum
    }
}

/// Specifies layer assignment for the given mixnode.
#[cw_serde]
pub struct LayerAssignment {
    /// The id of the mixnode.
    mix_id: MixId,

    /// The layer to which it's going to be assigned
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

/// The current layer distribution of the mix network.
#[cw_serde]
#[derive(Copy, Default)]
pub struct LayerDistribution {
    /// Number of nodes on the first layer.
    pub layer1: u64,

    /// Number of nodes on the second layer.
    pub layer2: u64,

    /// Number of nodes on the third layer.
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

/// The current state of the mixnet contract.
#[cw_serde]
pub struct ContractState {
    /// Address of the contract owner.
    #[deprecated(
        note = "use explicit ADMIN instead. this field will be removed in future release"
    )]
    #[serde(default)]
    pub owner: Option<Addr>,

    /// Address of "rewarding validator" (nym-api) that's allowed to send any rewarding-related transactions.
    pub rewarding_validator_address: Addr,

    /// Address of the vesting contract to which the mixnet contract would be sending all
    /// track-related messages.
    pub vesting_contract_address: Addr,

    /// The expected denom used for rewarding (and realistically any other operation).
    /// Default: `unym`
    pub rewarding_denom: String,

    /// Contract parameters that could be adjusted in a transaction the contract admin.
    pub params: ContractStateParams,
}

/// Contract parameters that could be adjusted in a transaction by the contract admin.
#[cw_serde]
pub struct ContractStateParams {
    /// Minimum amount a delegator must stake in orders for his delegation to get accepted.
    pub minimum_mixnode_delegation: Option<Coin>,

    /// Minimum amount a mixnode must pledge to get into the system.
    pub minimum_mixnode_pledge: Coin,

    /// Minimum amount a gateway must pledge to get into the system.
    pub minimum_gateway_pledge: Coin,

    /// Defines the allowed profit margin range of operators.
    /// default: 0% - 100%
    #[serde(default)]
    pub profit_margin: ProfitMarginRange,

    /// Defines the allowed interval operating cost range of operators.
    /// default: 0 - 1'000'000'000'000'000 (1 Billion native tokens - the total supply)
    #[serde(default)]
    pub interval_operating_cost: OperatingCostRange,
}
