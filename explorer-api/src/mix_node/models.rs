// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::Cache;
use crate::mix_nodes::location::Location;
use nym_contracts_common::Percent;
use nym_mixnet_contract_common::Delegation;
use nym_mixnet_contract_common::{Addr, Coin, Layer, MixId, MixNode};
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use validator_client::models::{NodePerformance, SelectionChance};

#[derive(Clone, Debug, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MixnodeStatus {
    Active,   // in both the active set and the rewarded set
    Standby,  // only in the rewarded set
    Inactive, // in neither the rewarded set nor the active set
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct PrettyDetailedMixNodeBond {
    // I leave this to @MS to refactor this type as a lot of things here are redundant thanks to
    // the existence of `MixNodeDetails`
    pub mix_id: MixId,
    pub location: Option<Location>,
    pub status: MixnodeStatus,
    pub pledge_amount: Coin,
    pub total_delegation: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub mix_node: MixNode,
    pub stake_saturation: f32,
    pub uncapped_saturation: f32,
    pub avg_uptime: u8,
    pub node_performance: NodePerformance,
    pub estimated_operator_apy: f64,
    pub estimated_delegators_apy: f64,
    pub operating_cost: Coin,
    pub profit_margin_percent: Percent,
    pub family_id: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct SummedDelegations {
    pub owner: Addr,
    pub mix_id: MixId,
    pub amount: Coin,
}

impl SummedDelegations {
    pub fn from(delegations: &[Delegation]) -> Option<Self> {
        let owner = get_common_owner(delegations)?;
        let mix_id = get_common_mix_id(delegations)?;
        let denom = get_common_denom(delegations)?;

        let sum = delegations
            .iter()
            .map(|delegation| delegation.amount.amount)
            .sum();

        let amount = Coin { denom, amount: sum };

        Some(SummedDelegations {
            owner,
            mix_id,
            amount,
        })
    }
}

pub(crate) struct MixNodeCache {
    pub(crate) descriptions: Cache<MixId, NodeDescription>,
    pub(crate) node_stats: Cache<MixId, NodeStats>,
    pub(crate) econ_stats: Cache<MixId, EconomicDynamicsStats>,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeMixNodeCache {
    inner: Arc<RwLock<MixNodeCache>>,
}

impl ThreadsafeMixNodeCache {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodeCache {
            inner: Arc::new(RwLock::new(MixNodeCache {
                descriptions: Cache::new(),
                node_stats: Cache::new(),
                econ_stats: Cache::new(),
            })),
        }
    }

    pub(crate) async fn get_description(&self, mix_id: MixId) -> Option<NodeDescription> {
        self.inner.read().await.descriptions.get(&mix_id)
    }

    pub(crate) async fn get_node_stats(&self, mix_id: MixId) -> Option<NodeStats> {
        self.inner.read().await.node_stats.get(&mix_id)
    }

    pub(crate) async fn get_econ_stats(&self, mix_id: MixId) -> Option<EconomicDynamicsStats> {
        self.inner.read().await.econ_stats.get(&mix_id)
    }

    pub(crate) async fn set_description(&self, mix_id: MixId, description: NodeDescription) {
        self.inner
            .write()
            .await
            .descriptions
            .set(mix_id, description);
    }

    pub(crate) async fn set_node_stats(&self, mix_id: MixId, node_stats: NodeStats) {
        self.inner.write().await.node_stats.set(mix_id, node_stats);
    }

    pub(crate) async fn set_econ_stats(&self, mix_id: MixId, econ_stats: EconomicDynamicsStats) {
        self.inner.write().await.econ_stats.set(mix_id, econ_stats);
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub(crate) struct NodeDescription {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) link: String,
    pub(crate) location: String,
}

#[derive(Serialize, Clone, Deserialize, JsonSchema)]
pub(crate) struct NodeStats {
    #[serde(
        serialize_with = "humantime_serde::serialize",
        deserialize_with = "humantime_serde::deserialize"
    )]
    update_time: SystemTime,

    #[serde(
        serialize_with = "humantime_serde::serialize",
        deserialize_with = "humantime_serde::deserialize"
    )]
    previous_update_time: SystemTime,

    packets_received_since_startup: u64,
    packets_sent_since_startup: u64,
    packets_explicitly_dropped_since_startup: u64,
    packets_received_since_last_update: u64,
    packets_sent_since_last_update: u64,
    packets_explicitly_dropped_since_last_update: u64,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct EconomicDynamicsStats {
    pub(crate) stake_saturation: f32,
    pub(crate) uncapped_saturation: f32,

    pub(crate) active_set_inclusion_probability: SelectionChance,
    pub(crate) reserve_set_inclusion_probability: SelectionChance,

    pub(crate) estimated_total_node_reward: u64,
    pub(crate) estimated_operator_reward: u64,
    pub(crate) estimated_delegators_reward: u64,

    pub(crate) current_interval_uptime: u8,
}

fn get_common_owner(delegations: &[Delegation]) -> Option<Addr> {
    let owner = delegations.iter().next()?.owner.clone();
    if delegations
        .iter()
        .any(|delegation| delegation.owner != owner)
    {
        log::warn!("Unexpected different owners when summing delegations");
        return None;
    }
    Some(owner)
}

fn get_common_mix_id(delegations: &[Delegation]) -> Option<MixId> {
    let mix_id = delegations.iter().next()?.mix_id;
    if delegations
        .iter()
        .any(|delegation| delegation.mix_id != mix_id)
    {
        log::warn!("Unexpected different node identities when summing delegations");
        return None;
    }
    Some(mix_id)
}

fn get_common_denom(delegations: &[Delegation]) -> Option<String> {
    let denom = delegations.iter().next()?.amount.denom.clone();
    if delegations
        .iter()
        .any(|delegation| delegation.amount.denom != denom)
    {
        log::warn!("Unexpected different coin denom when summing delegations");
        return None;
    }
    Some(denom)
}
