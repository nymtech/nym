// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::DelegatorRewardParams;
use crate::Layer;
use cosmwasm_std::{Addr, Uint128};
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
pub struct ContractStateParams {
    // so currently epoch_length is being unused and validator API performs rewarding
    // based on its own epoch length config value. I guess that's fine for time being
    // however, in the future, the contract constant should be controlling it instead.
    // pub epoch_length: u32, // length of a rewarding epoch/interval, expressed in hours
    pub minimum_mixnode_pledge: Uint128, // minimum amount a mixnode must pledge to get into the system
    pub minimum_gateway_pledge: Uint128, // minimum amount a gateway must pledge to get into the system

    // number of mixnode that are going to get rewarded during current rewarding interval (k_m)
    // based on overall demand for private bandwidth-
    pub mixnode_rewarded_set_size: u32,

    // subset of rewarded mixnodes that are actively receiving mix traffic
    // used to handle shorter-term (e.g. hourly) fluctuations of demand
    pub mixnode_active_set_size: u32,
    pub active_set_work_factor: u8,
}

impl Display for ContractStateParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Contract state parameters: [ ")?;
        write!(
            f,
            "minimum mixnode pledge: {}; ",
            self.minimum_mixnode_pledge
        )?;
        write!(
            f,
            "minimum gateway pledge: {}; ",
            self.minimum_gateway_pledge
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
        )?;
        write!(
            f,
            "mixnode active set work factor: {}",
            self.active_set_work_factor
        )
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct RewardingResult {
    pub operator_reward: Uint128,
    pub total_delegator_reward: Uint128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PendingDelegatorRewarding {
    // keep track of the running rewarding results so we'd known how much was the operator and its delegators rewarded
    pub running_results: RewardingResult,

    pub next_start: Addr,

    pub rewarding_params: DelegatorRewardParams,
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
