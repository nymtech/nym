// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_families::cache::{
    BlockTime, CachedFamilyBuilder, CachedFamilyMember, NodeFamiliesCacheData,
};
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
use tracing::{debug, error};

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

    /// Blocks to look back when bootstrapping an average block time for
    /// estimating timestamps of pruned heights (used only when no anchors exist).
    block_time_estimation_lookback: u32,
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
        block_time_estimation_lookback: u32,
        nyxd_client: Client,
        mixnet_contract_cache: MixnetContractCache,
        shared_cache: SharedCache<NodeFamiliesCacheData>,
    ) -> Self {
        NodeFamiliesDataProvider {
            nyxd_client,
            mixnet_contract_cache,
            shared_cache,
            block_timestamp_fetch_concurrency,
            block_time_estimation_lookback,
        }
    }

    /// Snapshot of the previously-cached block timestamps (rehydrated from
    /// disk on startup). Empty if the cache hasn't been initialised yet.
    async fn previous_block_timestamps(&self) -> HashMap<u64, BlockTime> {
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
    /// from the previous cache (fetched or estimated), parallel-fetch the rest,
    /// and estimate any height the RPC node has already pruned.
    async fn resolve_block_timestamps(
        &self,
        referenced_heights: &HashSet<u64>,
    ) -> HashMap<u64, BlockTime> {
        let mut block_timestamps = self.previous_block_timestamps().await;

        // anything already present (fetched *or* estimated) is skipped, so
        // pruned heights are never re-queried once estimated.
        let to_fetch: Vec<u64> = referenced_heights
            .iter()
            .filter(|h| !block_timestamps.contains_key(h))
            .copied()
            .collect();

        let FetchedTimestamps { fetched, pruned } = self.fetch_block_timestamps(to_fetch).await;
        for anchor in fetched {
            block_timestamps.insert(anchor.height, BlockTime::Fetched(anchor.time));
        }

        if !pruned.is_empty() {
            let estimates = self.estimate_block_times(&pruned, &block_timestamps).await;
            debug!(
                "estimated timestamps for {}/{} pruned block height(s)",
                estimates.len(),
                pruned.len()
            );
            for (h, t) in estimates {
                block_timestamps.insert(h, BlockTime::Estimated(t));
            }
        }

        block_timestamps
    }

    /// Parallel-fetch the timestamps for the given heights, partitioning the
    /// results into those served by the RPC node and those it has pruned.
    /// Transient failures are logged and dropped (retried next tick).
    async fn fetch_block_timestamps(&self, heights: Vec<u64>) -> FetchedTimestamps {
        enum Outcome {
            Fetched(BlockAnchor),
            Pruned(u64),
        }

        let outcomes: Vec<Outcome> = stream::iter(heights)
            .map(|h| async move {
                match self.nyxd_client.block_timestamp(h as u32).await {
                    Ok(t) => Some(Outcome::Fetched(BlockAnchor::new(
                        h,
                        OffsetDateTime::from(t),
                    ))),
                    // the block has been pruned by the connected RPC node and
                    // can never be served - estimate it instead of dropping it.
                    Err(err) if err.is_block_pruned() => Some(Outcome::Pruned(h)),
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

        let mut result = FetchedTimestamps::default();
        for outcome in outcomes {
            match outcome {
                Outcome::Fetched(anchor) => result.fetched.push(anchor),
                Outcome::Pruned(h) => result.pruned.push(h),
            }
        }
        result
    }

    /// Estimate timestamps for pruned heights by extrapolating from each
    /// height's nearest anchor at the chain's average block time. Returns an
    /// empty map when no block time can be established (those heights then stay
    /// unresolved - no worse than dropping them).
    async fn estimate_block_times(
        &self,
        pruned: &[u64],
        known: &HashMap<u64, BlockTime>,
    ) -> HashMap<u64, OffsetDateTime> {
        // only real (fetched) entries are trustworthy anchors; never extrapolate
        // off another estimate.
        let mut anchors: Vec<BlockAnchor> = known
            .iter()
            .filter_map(|(h, bt)| match bt {
                BlockTime::Fetched(t) => Some(BlockAnchor::new(*h, *t)),
                BlockTime::Estimated(_) => None,
            })
            .collect();
        anchors.sort_by_key(|a| a.height);

        let Some(model) = self.resolve_block_time_model(&anchors).await else {
            return HashMap::new();
        };

        estimate_from_anchors(pruned, &model.anchors, model.block_time_secs)
    }

    /// Establish the average block time and the anchor set to extrapolate from.
    /// `anchors` must be sorted ascending by height.
    async fn resolve_block_time_model(&self, anchors: &[BlockAnchor]) -> Option<BlockTimeModel> {
        // >= 2 anchors: derive block time from their span; each pruned height is
        // later extrapolated from its *nearest* anchor. No RPC needed.
        if let Some(block_time_secs) = average_block_time_secs(anchors) {
            return Some(BlockTimeModel {
                block_time_secs,
                anchors: anchors.to_vec(),
            });
        }

        // fewer than 2 usable anchors: bring in the current block as a reference.
        let tip = self.current_block().await?;

        match anchors.first() {
            // exactly one anchor: pair it with the chain tip for block time; both
            // become anchors (the tip is always above any pruned height).
            Some(&anchor) => {
                let block_time_secs = average_block_time_secs(&[anchor, tip])?;
                Some(BlockTimeModel {
                    block_time_secs,
                    anchors: vec![anchor, tip],
                })
            }
            // no anchors: use the current block plus a block `lookback` heights
            // earlier to derive the block time.
            None => {
                let earlier_height = tip
                    .height
                    .checked_sub(self.block_time_estimation_lookback as u64)?;
                let earlier = BlockAnchor::new(
                    earlier_height,
                    self.block_timestamp_at(earlier_height).await?,
                );
                let block_time_secs = average_block_time_secs(&[earlier, tip])?;
                Some(BlockTimeModel {
                    block_time_secs,
                    anchors: vec![earlier, tip],
                })
            }
        }
    }

    /// Current block anchor (height + time), or `None` on RPC failure.
    async fn current_block(&self) -> Option<BlockAnchor> {
        match self.nyxd_client.current_block_info().await {
            Ok(block) => Some(BlockAnchor::new(
                block.block.header.height.value(),
                OffsetDateTime::from(block.block.header.time),
            )),
            Err(err) => {
                error!("failed to retrieve current block info for timestamp estimation: {err}");
                None
            }
        }
    }

    /// Timestamp of a specific (recent, unpruned) height, or `None` on failure.
    async fn block_timestamp_at(&self, height: u64) -> Option<OffsetDateTime> {
        match self.nyxd_client.block_timestamp(height as u32).await {
            Ok(t) => Some(OffsetDateTime::from(t)),
            Err(err) => {
                error!("failed to retrieve reference block timestamp at height {height}: {err}");
                None
            }
        }
    }
}

/// A block height paired with its timestamp; the unit of block-time estimation.
#[derive(Copy, Clone)]
struct BlockAnchor {
    height: u64,
    time: OffsetDateTime,
}

impl BlockAnchor {
    fn new(height: u64, time: OffsetDateTime) -> Self {
        BlockAnchor { height, time }
    }
}

/// Result of fetching a batch of block timestamps: those served by the RPC
/// node, and those it has already pruned.
#[derive(Default)]
struct FetchedTimestamps {
    fetched: Vec<BlockAnchor>,
    pruned: Vec<u64>,
}

/// Average block time plus the (sorted, non-empty) anchor set to extrapolate
/// each pruned height from.
struct BlockTimeModel {
    block_time_secs: f64,
    anchors: Vec<BlockAnchor>,
}

/// Average seconds per block across the given anchors. Needs at least 2 anchors
/// spanning distinct heights and a positive elapsed time; `None` otherwise.
fn average_block_time_secs(anchors: &[BlockAnchor]) -> Option<f64> {
    if anchors.len() < 2 {
        return None;
    }
    let oldest = anchors.iter().min_by_key(|a| a.height).copied()?;
    let newest = anchors.iter().max_by_key(|a| a.height).copied()?;
    let height_span = newest.height.checked_sub(oldest.height)?;
    if height_span == 0 {
        return None;
    }
    let elapsed = (newest.time - oldest.time).as_seconds_f64();
    if elapsed <= 0.0 {
        return None;
    }
    Some(elapsed / height_span as f64)
}

/// Map each pruned height to an estimated timestamp, extrapolating from its
/// nearest anchor. `anchors` must be sorted ascending; heights with no anchor
/// (empty set) are skipped.
fn estimate_from_anchors(
    pruned: &[u64],
    anchors: &[BlockAnchor],
    block_time_secs: f64,
) -> HashMap<u64, OffsetDateTime> {
    pruned
        .iter()
        .filter_map(|&h| {
            let base = nearest_anchor(anchors, h)?;
            Some((h, extrapolate_timestamp(base, h, block_time_secs)))
        })
        .collect()
}

/// Anchor closest in height to `target_height`. `anchors` must be sorted
/// ascending by height; `None` only if empty. Ties favour the lower anchor.
fn nearest_anchor(anchors: &[BlockAnchor], target_height: u64) -> Option<BlockAnchor> {
    let idx = anchors.partition_point(|a| a.height < target_height);
    // the nearest is one of the neighbours straddling the target: `idx - 1`
    // (highest below) or `idx` (lowest at-or-above).
    [idx.checked_sub(1), Some(idx)]
        .into_iter()
        .flatten()
        .filter_map(|i| anchors.get(i).copied())
        .min_by_key(|a| a.height.abs_diff(target_height))
}

/// Extrapolate the timestamp of `target_height` from a base anchor at the given
/// average block time. Signed: targets above the anchor land later in time,
/// targets below land earlier.
fn extrapolate_timestamp(
    base: BlockAnchor,
    target_height: u64,
    block_time_secs: f64,
) -> OffsetDateTime {
    let delta_blocks = base.height as i64 - target_height as i64;
    base.time - time::Duration::seconds_f64(delta_blocks as f64 * block_time_secs)
}

/// Average member age: for each member with a known bonding
/// height we have a cached block-time, take `now - t` and average. Heights we
/// failed to resolve are skipped rather than poisoning the average.
fn average_node_age(
    members: &[CachedFamilyMember],
    block_timestamps: &HashMap<u64, BlockTime>,
) -> Duration {
    let now = OffsetDateTime::now_utc();
    let mut total_secs: i64 = 0;
    let mut count: i64 = 0;
    for height in members.iter().filter_map(|m| m.bonding_height) {
        let Some(ts) = block_timestamps.get(&height) else {
            continue;
        };
        let age = (now - ts.time()).whole_seconds();
        if age < 0 {
            continue;
        }
        total_secs += age;
        count += 1;
    }
    if count == 0 {
        return Duration::ZERO;
    }
    Duration::from_secs((total_secs / count) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(unix: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(unix).unwrap()
    }

    fn anchor(height: u64, unix: i64) -> BlockAnchor {
        BlockAnchor::new(height, dt(unix))
    }

    #[test]
    fn average_block_time_needs_two_distinct_anchors() {
        assert!(average_block_time_secs(&[]).is_none());
        assert!(average_block_time_secs(&[anchor(100, 1000)]).is_none());
        // same height twice => zero span
        assert!(average_block_time_secs(&[anchor(100, 1000), anchor(100, 1060)]).is_none());
    }

    #[test]
    fn average_block_time_from_span() {
        // 10 blocks over 60s => 6s/block, order-independent
        let secs = average_block_time_secs(&[anchor(100, 1000), anchor(110, 1060)]).unwrap();
        assert!((secs - 6.0).abs() < 1e-9);
        let reversed = average_block_time_secs(&[anchor(110, 1060), anchor(100, 1000)]).unwrap();
        assert!((reversed - 6.0).abs() < 1e-9);
    }

    #[test]
    fn average_block_time_rejects_nonpositive_elapsed() {
        // identical timestamps => zero elapsed
        assert!(average_block_time_secs(&[anchor(100, 1000), anchor(110, 1000)]).is_none());
        // time decreasing with height => negative elapsed
        assert!(average_block_time_secs(&[anchor(100, 1060), anchor(110, 1000)]).is_none());
    }

    #[test]
    fn extrapolate_backward_basic() {
        // base height 200 @ t=2000, 6s/block, target 100 => 100*6 = 600s earlier
        assert_eq!(extrapolate_timestamp(anchor(200, 2000), 100, 6.0), dt(1400));
    }

    #[test]
    fn extrapolate_uses_real_prune_heights() {
        // heights from the observed prune error: pruned 14_862_522, lowest
        // available (anchor) 16_853_136.
        let base = anchor(16_853_136, 2_000_000_000);
        let target_h = 14_862_522u64;
        let block_time = 6.0;
        let expected =
            base.time - time::Duration::seconds_f64((base.height - target_h) as f64 * block_time);
        assert_eq!(extrapolate_timestamp(base, target_h, block_time), expected);
        // and the estimate must be strictly older than the anchor
        assert!(extrapolate_timestamp(base, target_h, block_time) < base.time);
    }

    #[test]
    fn extrapolate_forward_when_target_above_base() {
        // target above the base anchor => later in time (signed extrapolation)
        assert_eq!(extrapolate_timestamp(anchor(100, 1000), 110, 6.0), dt(1060));
    }

    #[test]
    fn nearest_anchor_picks_closest_by_height() {
        let anchors = [anchor(100, 0), anchor(200, 0), anchor(400, 0)];
        assert_eq!(nearest_anchor(&anchors, 90).unwrap().height, 100); // below all
        assert_eq!(nearest_anchor(&anchors, 140).unwrap().height, 100); // closer to 100
        assert_eq!(nearest_anchor(&anchors, 160).unwrap().height, 200); // closer to 200
        assert_eq!(nearest_anchor(&anchors, 200).unwrap().height, 200); // exact match
        assert_eq!(nearest_anchor(&anchors, 500).unwrap().height, 400); // above all
        assert!(nearest_anchor(&[], 100).is_none());
    }

    #[test]
    fn estimates_pruned_height_above_oldest_cached_anchor() {
        // regression: a Fetched anchor cached from an earlier run (14_000_000)
        // persists below a newly-pruned referenced height (14_500_000); the prune
        // boundary has since advanced to 16_853_136 (also fetched). The old code
        // dropped any pruned height >= the oldest anchor; it must now be estimated
        // from its nearest anchor instead.
        let block_time = 6.0;
        let old = anchor(14_000_000, 1_000_000_000);
        let boundary = anchor(16_853_136, 1_000_000_000 + (16_853_136 - 14_000_000) * 6);
        let anchors = [old, boundary]; // sorted ascending
        let pruned = [14_500_000u64];

        let estimates = estimate_from_anchors(&pruned, &anchors, block_time);
        let estimated = estimates
            .get(&14_500_000)
            .copied()
            .expect("pruned height above the oldest anchor must still be estimated");

        // nearest anchor is `old` (Δ 500_000) not `boundary` (Δ 2_353_136);
        // 14_500_000 is 500_000 blocks *after* `old`.
        let expected = old.time + time::Duration::seconds_f64(500_000.0 * block_time);
        assert_eq!(estimated, expected);
        // and it lands between the two anchors in time
        assert!(estimated > old.time && estimated < boundary.time);
    }
}
