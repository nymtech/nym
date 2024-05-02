// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::Cache;
use nym_mixnet_contract_common::Delegation;
use nym_mixnet_contract_common::{Addr, Coin, MixId};
use nym_validator_client::models::SelectionChance;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
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
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) link: Option<String>,
    pub(crate) location: Option<String>,
    pub(crate) moniker: Option<String>,
    pub(crate) website: Option<String>,
    pub(crate) security_contact: Option<String>,
    pub(crate) details: Option<String>,
}

#[derive(Deserialize)]
struct OldModelDescription {
    name: String,
    description: String,
    link: String,
    location: String,
}

#[derive(Deserialize)]
struct NewModelDescription {
    moniker: String,
    website: String,
    security_contact: String,
    details: String,
}

impl From<OldModelDescription> for NodeDescription {
    fn from(old: OldModelDescription) -> Self {
        NodeDescription {
            name: Some(old.name),
            description: Some(old.description),
            link: Some(old.link),
            location: Some(old.location),
            moniker: None,
            website: None,
            security_contact: None,
            details: None,
        }
    }
}

impl From<NewModelDescription> for NodeDescription {
    fn from(new: NewModelDescription) -> Self {
        NodeDescription {
            name: None,
            description: Some(new.details),
            link: Some(new.website),
            location: None,
            moniker: Some(new.moniker),
            website: None,
            security_contact: Some(new.security_contact),
            details: None,
        }
    }
}

#[derive(Serialize, Clone, Deserialize, JsonSchema, Debug)]
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

    #[serde(alias = "received_since_startup")]
    packets_received_since_startup: u64,
    #[serde(alias = "sent_since_startup")]
    packets_sent_since_startup: u64,
    #[serde(alias = "dropped_since_startup")]
    packets_explicitly_dropped_since_startup: u64,
    #[serde(alias = "received_since_last_update")]
    packets_received_since_last_update: u64,
    #[serde(alias = "sent_since_last_update")]
    packets_sent_since_last_update: u64,
    #[serde(alias = "dropped_since_last_update")]
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
