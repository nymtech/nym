// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_families::cache::{
    CachedFamily, CachedFamilyBuilder, CachedFamilyMember, NodeFamiliesCacheData,
};
use crate::support::caching::refresher::CacheItemProvider;
use crate::support::nyxd::Client;
use async_trait::async_trait;
use nym_validator_client::nyxd::contract_traits::PagedNodeFamiliesQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use std::collections::HashMap;
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
    pub(crate) fn new(nyxd_client: Client, mixnet_contract_cache: MixnetContractCache) -> Self {
        NodeFamiliesDataProvider {
            nyxd_client,
            mixnet_contract_cache,
        }
    }

    /// Approximate average member age, derived from a single timestamp lookup
    /// at the mean bonding height. Height-weighted (not time-weighted) so the
    /// chain RPC count stays O(1) per family instead of O(members).
    async fn average_node_age(&self, members: &[CachedFamilyMember]) -> Duration {
        let mut known_heights = 0;
        let mut running_total = 0;

        for bonding_height in members.iter().filter_map(|m| m.bonding_height) {
            known_heights += 1;
            running_total += bonding_height;
        }

        if known_heights == 0 {
            return Duration::ZERO;
        }

        let block_height = running_total / known_heights;
        let Ok(timestamp) = self.nyxd_client.block_timestamp(block_height as u32).await else {
            error!("failed to retrieve block timestamp for block height: {block_height}");
            return Duration::ZERO;
        };

        let t: OffsetDateTime = timestamp.into();
        (OffsetDateTime::now_utc() - t).unsigned_abs()
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

        let mut families: HashMap<_, CachedFamilyBuilder> = HashMap::new();
        for family in raw_families {
            families.insert(family.id, family.into());
        }

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
                .push(CachedFamilyMember::new(member_record, node_info))
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

        let mut family_details = Vec::new();

        for family in families.into_values() {
            let average_node_age = self.average_node_age(&family.members).await;
            family_details.push(family.build(average_node_age))
        }

        Ok(NodeFamiliesCacheData {
            families: family_details,
        })
    }
}
