// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_families::cache::{CachedFamilyBuilder, CachedFamilyMember, NodeFamiliesCacheData};
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::CacheItemProvider;
use crate::support::nyxd::Client;
use async_trait::async_trait;
use futures::{stream, StreamExt};
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::contract_traits::PagedNodeFamiliesQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Duration;
use time::OffsetDateTime;
use tracing::error;

/// Periodic refresher feeding the [`NodeFamiliesCacheData`] cache from the
/// node-families contract, joined with mixnet-contract stake snapshots.
pub struct NodeFamiliesDataProvider {
    /// Nyxd client used for contract queries and block timestamp lookups.
    nyxd_client: Client,

    /// Source of per-node stake/delegation information.
    mixnet_contract_cache: MixnetContractCache,

    /// Read-only handle to the cache this provider feeds. Used to recover the
    /// previously-known block-height → block-time map (rehydrated from disk on
    /// startup) so we only RPC heights we haven't already seen.
    shared_cache: SharedCache<NodeFamiliesCacheData>,

    /// Maximum number of `block_timestamp` lookups in flight in parallel during a
    /// single refresh tick.
    block_timestamp_fetch_concurrency: usize,
}

#[async_trait]
impl CacheItemProvider for NodeFamiliesDataProvider {
    type Item = NodeFamiliesCacheData;
    type Error = NyxdError;

    async fn wait_until_ready(&self) {
        self.mixnet_contract_cache
            .naive_wait_for_initial_values()
            .await
    }

    async fn try_refresh(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        self.refresh().await.map(Some)
    }
}

impl NodeFamiliesDataProvider {
    pub(crate) fn new(
        block_timestamp_fetch_concurrency: usize,
        nyxd_client: Client,
        mixnet_contract_cache: MixnetContractCache,
        shared_cache: SharedCache<NodeFamiliesCacheData>,
    ) -> Self {
        NodeFamiliesDataProvider {
            nyxd_client,
            mixnet_contract_cache,
            shared_cache,
            block_timestamp_fetch_concurrency,
        }
    }

    /// Snapshot of the previously-cached block timestamps (rehydrated from
    /// disk on startup). Empty if the cache hasn't been initialised yet.
    async fn previous_block_timestamps(&self) -> HashMap<u64, OffsetDateTime> {
        let Ok(prev) = self.shared_cache.get().await else {
            return HashMap::new();
        };
        prev.block_timestamps.clone()
    }

    /// Pull the full families/members/pending-invitations snapshot from the
    /// node-families contract and join it with the latest mixnet-contract node
    /// information for stake/bonding data.
    async fn refresh(&self) -> Result<NodeFamiliesCacheData, NyxdError> {
        // retrieve the base data from the contract
        let raw_families = self.nyxd_client.get_all_families().await?;
        let raw_members = self.nyxd_client.get_all_family_members().await?;
        let pending_invites = self.nyxd_client.get_all_pending_invitations().await?;

        let nym_nodes = self
            .mixnet_contract_cache
            .nym_nodes()
            .await
            .into_iter()
            .map(|node| (node.node_id(), node))
            .collect::<HashMap<_, _>>();

        let mut families: HashMap<_, CachedFamilyBuilder> = raw_families
            .into_iter()
            .map(|family| (family.id, family.into()))
            .collect();
        let mut family_by_member: HashMap<NodeId, _> = HashMap::new();

        // insert all member information into appropriate families
        for member_record in raw_members {
            let family_id = member_record.membership.family_id;
            let node_id = member_record.node_id;
            let Some(family) = families.get_mut(&family_id) else {
                error!(
                    "node {node_id} belongs to family {family_id}, but this family does not exist!",
                );
                continue;
            };
            let node_info = nym_nodes.get(&node_id);
            family
                .members
                .push(CachedFamilyMember::new(member_record, node_info));
            family_by_member.insert(node_id, family_id);
        }

        // insert all invitations into appropriate families
        for invitation in pending_invites {
            let family_id = invitation.invitation.family_id;
            let node_id = invitation.invitation.node_id;
            let Some(family) = families.get_mut(&family_id) else {
                error!(
                    "node {node_id} has been invited to family {family_id}, but this family does not exist!",
                );
                continue;
            };
            family.pending_invitations.push(invitation.into());
        }

        let referenced_heights: HashSet<u64> = families
            .values()
            .flat_map(|f| f.members.iter().filter_map(|m| m.bonding_height))
            .collect();

        let block_timestamps = self.resolve_block_timestamps(&referenced_heights).await;

        let family_details: BTreeMap<_, _> = families
            .into_values()
            .map(|family| {
                let average_node_age = average_node_age(&family.members, &block_timestamps);
                let built = family.build(average_node_age);
                (built.id, built)
            })
            .collect();

        Ok(NodeFamiliesCacheData {
            families: family_details,
            family_by_member,
            block_timestamps,
        })
    }

    /// Build the block-height → block-time map for this refresh: keep entries
    /// from the previous cache that we still need, parallel-fetch the rest.
    async fn resolve_block_timestamps(
        &self,
        referenced_heights: &HashSet<u64>,
    ) -> HashMap<u64, OffsetDateTime> {
        let mut block_timestamps = self.previous_block_timestamps().await;

        let to_fetch: Vec<u64> = referenced_heights
            .iter()
            .filter(|h| !block_timestamps.contains_key(h))
            .copied()
            .collect();

        let fetched: Vec<(u64, OffsetDateTime)> = stream::iter(to_fetch)
            .map(|h| async move {
                match self.nyxd_client.block_timestamp(h as u32).await {
                    Ok(t) => Some((h, OffsetDateTime::from(t))),
                    Err(err) => {
                        error!("failed to retrieve block timestamp for height {h}: {err}");
                        None
                    }
                }
            })
            .buffer_unordered(self.block_timestamp_fetch_concurrency)
            .filter_map(|x| async move { x })
            .collect()
            .await;

        block_timestamps.extend(fetched);
        block_timestamps
    }
}

/// Time-weighted average member age: for each member with a known bonding
/// height we have a cached block-time, take `now - t` and average. Heights we
/// failed to resolve are skipped rather than poisoning the average.
fn average_node_age(
    members: &[CachedFamilyMember],
    block_timestamps: &HashMap<u64, OffsetDateTime>,
) -> Duration {
    let now = OffsetDateTime::now_utc();
    let mut total_secs: i64 = 0;
    let mut count: i64 = 0;
    for height in members.iter().filter_map(|m| m.bonding_height) {
        let Some(ts) = block_timestamps.get(&height) else {
            continue;
        };
        total_secs += (now - *ts).whole_seconds();
        count += 1;
    }
    if count == 0 {
        return Duration::ZERO;
    }
    Duration::from_secs((total_secs / count).max(0) as u64)
}
