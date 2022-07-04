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

pub type EpochId = u32;
pub type NodeId = u64;

/// Percent represents a value between 0 and 100%
/// (i.e. between 0.0 and 1.0)
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
pub struct Percent(#[serde(deserialize_with = "de_decimal_percent")] Decimal);

impl Percent {
    pub fn new(value: Decimal) -> Result<Self, MixnetContractError> {
        if value > Decimal::one() {
            Err(MixnetContractError::InvalidPercent)
        } else {
            Ok(Percent(value))
        }
    }

    // essentially allows the TryFrom u8, u16, u32, u64, etc
    pub fn from_percentage_value<P: Into<u64>>(value: P) -> Result<Self, MixnetContractError> {
        Percent::new(Decimal::percent(value.into()))
    }

    pub fn value(&self) -> Decimal {
        self.0
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

    pub fn increment_layer_count(&mut self, layer: Layer) {
        match layer {
            Layer::Gateway => self.gateways += 1,
            Layer::One => self.layer1 += 1,
            Layer::Two => self.layer2 += 1,
            Layer::Three => self.layer3 += 1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractStateParams {
    pub minimum_mixnode_pledge: Coin, // minimum amount a mixnode must pledge to get into the system
    pub minimum_gateway_pledge: Coin, // minimum amount a gateway must pledge to get into the system

                                      // // number of mixnode that are going to get rewarded during current rewarding interval (k_m)
                                      // // based on overall demand for private bandwidth-
                                      // pub mixnode_rewarded_set_size: u32,
                                      //
                                      // // subset of rewarded mixnodes that are actively receiving mix traffic
                                      // // used to handle shorter-term (e.g. hourly) fluctuations of demand
                                      // pub mixnode_active_set_size: u32,
                                      // pub staking_supply: Uint128,
}

impl Display for ContractStateParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Contract state parameters: ")?;
        write!(
            f,
            "minimum mixnode pledge: {}; ",
            self.minimum_mixnode_pledge
        )?;
        write!(
            f,
            "minimum gateway pledge: {}; ",
            self.minimum_gateway_pledge
        )
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
        let valid_value = Percent::from_percentage_value(80u32).unwrap();
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
            Percent::from_percentage_value(95u32).unwrap()
        )
    }
}
