// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
// use crate::mixnode::DelegatorRewardParams;
use crate::{Layer, RewardedSetNodeStatus};
use cosmwasm_std::{Addr, Uint128};
use cosmwasm_std::{Coin, Decimal};
use schemars::JsonSchema;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{self, Display, Formatter};
use std::ops::Mul;

pub type EpochId = u32;
pub type IntervalId = u32;
pub type NodeId = u64;

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

// /// Represents a base58-encoded ed25519 signature on the bech32 address of the node owner.
// #[derive(Debug, Serialize, Deserialize, Clone)]
// pub struct OwnershipSignature(String);
//
// impl OwnershipSignature {
//     pub fn from_bytes(bytes: [u8; 64]) -> Self {
//         OwnershipSignature(bs58::encode(bytes).into_string())
//     }
//
//     pub fn try_from_encoded_base58(raw: &str) -> Result<Self, MixnetContractError> {
//         // we cannot do much validation without importing appropriate crypto library
//         // (which we want to avoid at this point), but we can at least check for expected length
//         // as ed25519 signatures are 64byte long.
//         let decoded = bs58::decode(raw)
//             .into_vec()
//             .map_err(|err| MixnetContractError::MalformedEd25519Signature(err.to_string()))?;
//
//         if decoded.len() != 64 {
//             return Err(MixnetContractError::MalformedEd25519Signature(format!(
//                 "Too few bytes provided for the signature. Got: {}, expected: 64",
//                 decoded.len()
//             )));
//         }
//
//         Ok(OwnershipSignature(raw.into()))
//     }
// }

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractStateParams {
    /// Minimum amount a delegator must stake in orders for his delegation to get accepted.
    pub minimum_mixnode_delegation: Option<Coin>,

    /// Minimum amount a mixnode must pledge to get into the system.
    pub minimum_mixnode_pledge: Coin,

    /// Minimum amount a gateway must pledge to get into the system.
    pub minimum_gateway_pledge: Coin,

    /// Address of the vesting contract to which the mixnet contract would be sending all
    /// track-related messages.
    pub vesting_contract_address: Addr,
}

impl Display for ContractStateParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
        // write!(f, "Contract state parameters: ")?;
        // write!(
        //     f,
        //     "minimum mixnode pledge: {}; ",
        //     self.minimum_mixnode_pledge
        // )?;
        // write!(
        //     f,
        //     "minimum gateway pledge: {}; ",
        //     self.minimum_gateway_pledge
        // )?;
        // if let Some(minimum_delegation) = &self.minimum_mixnode_delegation {
        //     write!(f, "minimum delegation: {}; ", minimum_delegation)?;
        // }
        //
        // Ok(())
        // write!(
        //     f,
        //     "mixnode rewarded set size: {}",
        //     self.mixnode_rewarded_set_size
        // )?;
        // write!(
        //     f,
        //     "mixnode active set size: {}",
        //     self.mixnode_active_set_size
        // )
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RewardingResult {
    pub node_reward: Uint128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingDelegatorRewarding {
    // keep track of the running rewarding results so we'd known how much was the operator and its delegators rewarded
    pub running_results: RewardingResult,

    pub next_start: Addr,
    // pub rewarding_params: DelegatorRewardParams,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RewardingStatus {
    Complete(RewardingResult),
    PendingNextDelegatorPage(PendingDelegatorRewarding),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MixnodeRewardingStatusResponse {
    pub status: Option<RewardingStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MixnetContractVersion {
    // VERGEN_BUILD_TIMESTAMP
    pub build_timestamp: String,

    // VERGEN_BUILD_SEMVER
    pub build_version: String,

    // VERGEN_GIT_SHA
    pub commit_sha: String,

    // VERGEN_GIT_COMMIT_TIMESTAMP
    pub commit_timestamp: String,

    // VERGEN_GIT_BRANCH
    pub commit_branch: String,

    // VERGEN_RUSTC_SEMVER
    pub rustc_version: String,
}

// type aliases for better reasoning about available data
pub type IdentityKey = String;
pub type IdentityKeyRef<'a> = &'a str;
pub type SphinxKey = String;
pub type SphinxKeyRef<'a> = &'a str;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
pub struct PagedRewardedSetResponse {
    pub identities: Vec<(IdentityKey, RewardedSetNodeStatus)>,
    pub start_next_after: Option<IdentityKey>,
    pub at_height: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RewardedSetUpdateDetails {
    pub refresh_rate_blocks: u64,
    pub last_refreshed_block: u64,
    pub current_height: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, JsonSchema)]
pub struct IntervalRewardedSetHeightsResponse {
    pub interval_id: u32,
    pub heights: Vec<u64>,
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
}
