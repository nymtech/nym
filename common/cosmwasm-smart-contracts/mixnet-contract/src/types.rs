// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::rewarding::helpers::truncate_decimal;
use crate::{Layer, RewardedSetNodeStatus};
use cosmwasm_std::{Addr, Uint128};
use cosmwasm_std::{Coin, Decimal};
use schemars::JsonSchema;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{self, Display, Formatter};
use std::ops::{Index, Mul};

// type aliases for better reasoning about available data
pub type IdentityKey = String;
pub type IdentityKeyRef<'a> = &'a str;
pub type SphinxKey = String;
pub type SphinxKeyRef<'a> = &'a str;
pub type EpochId = u32;
pub type IntervalId = u32;
pub type NodeId = u64;
pub type EpochEventId = u32;
pub type IntervalEventId = u32;

/// Percent represents a value between 0 and 100%
/// (i.e. between 0.0 and 1.0)
#[derive(
    Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
pub struct Percent(#[serde(deserialize_with = "de_decimal_percent")] Decimal);

impl Percent {
    pub fn new(value: Decimal) -> Result<Self, MixnetContractError> {
        if value > Decimal::one() {
            Err(MixnetContractError::InvalidPercent)
        } else {
            Ok(Percent(value))
        }
    }

    pub fn is_zero(&self) -> bool {
        self.0 == Decimal::zero()
    }

    pub fn from_percentage_value(value: u64) -> Result<Self, MixnetContractError> {
        Percent::new(Decimal::percent(value))
    }

    pub fn value(&self) -> Decimal {
        self.0
    }

    pub fn round_to_integer(&self) -> u8 {
        let hundred = Decimal::from_ratio(100u32, 1u32);
        // we know the cast from u128 to u8 is a safe one since the internal value must be within 0 - 1 range
        truncate_decimal(hundred * self.0).u128() as u8
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let adjusted = Decimal::from_atomics(100u32, 0).unwrap() * self.0;
        write!(f, "{}%", adjusted)
    }
}

impl Mul<Decimal> for Percent {
    type Output = Decimal;

    fn mul(self, rhs: Decimal) -> Self::Output {
        self.0 * rhs
    }
}

impl Mul<Percent> for Decimal {
    type Output = Decimal;

    fn mul(self, rhs: Percent) -> Self::Output {
        rhs * self
    }
}

impl Mul<Uint128> for Percent {
    type Output = Uint128;

    fn mul(self, rhs: Uint128) -> Self::Output {
        self.0 * rhs
    }
}

// implement custom Deserialize because we want to validate Percent has the correct range
fn de_decimal_percent<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Decimal::deserialize(deserializer)?;
    if v > Decimal::one() {
        Err(D::Error::custom(
            "provided decimal percent is larger than 100%",
        ))
    } else {
        Ok(v)
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractState {
    pub owner: Addr, // only the owner account can update state
    pub rewarding_validator_address: Addr,

    /// Address of the vesting contract to which the mixnet contract would be sending all
    /// track-related messages.
    pub vesting_contract_address: Addr,
    pub rewarding_denom: String,
    pub params: ContractStateParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractStateParams {
    /// Minimum amount a delegator must stake in orders for his delegation to get accepted.
    pub minimum_mixnode_delegation: Option<Coin>,

    /// Minimum amount a mixnode must pledge to get into the system.
    pub minimum_mixnode_pledge: Coin,

    /// Minimum amount a gateway must pledge to get into the system.
    pub minimum_gateway_pledge: Coin,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct PagedRewardedSetResponse {
    pub nodes: Vec<(NodeId, RewardedSetNodeStatus)>,
    pub start_next_after: Option<NodeId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_serde() {
        let valid_value = Percent::from_percentage_value(80).unwrap();
        let serialized = serde_json::to_string(&valid_value).unwrap();

        println!("{}", serialized);
        let deserialized: Percent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(valid_value, deserialized);

        let invalid_values = vec!["\"42\"", "\"1.1\"", "\"1.00000001\"", "\"foomp\"", "\"1a\""];
        for invalid_value in invalid_values {
            assert!(serde_json::from_str::<'_, Percent>(invalid_value).is_err())
        }
        assert_eq!(
            serde_json::from_str::<'_, Percent>("\"0.95\"").unwrap(),
            Percent::from_percentage_value(95).unwrap()
        )
    }

    #[test]
    fn percent_to_absolute_integer() {
        let p = serde_json::from_str::<'_, Percent>("\"0.0001\"").unwrap();
        assert_eq!(p.round_to_integer(), 0);

        let p = serde_json::from_str::<'_, Percent>("\"0.0099\"").unwrap();
        assert_eq!(p.round_to_integer(), 0);

        let p = serde_json::from_str::<'_, Percent>("\"0.0199\"").unwrap();
        assert_eq!(p.round_to_integer(), 1);

        let p = serde_json::from_str::<'_, Percent>("\"0.45123\"").unwrap();
        assert_eq!(p.round_to_integer(), 45);

        let p = serde_json::from_str::<'_, Percent>("\"0.999999999\"").unwrap();
        assert_eq!(p.round_to_integer(), 99);

        let p = serde_json::from_str::<'_, Percent>("\"1.00\"").unwrap();
        assert_eq!(p.round_to_integer(), 100);
    }
}
