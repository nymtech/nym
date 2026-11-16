// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_nodes::node_view::NodeSubset;
use crate::support::attested_snapshots::data::RetainedHeight;
use crate::support::attested_snapshots::AVERAGE_BLOCK_TIME;
use nym_api_requests::models::described::v3::NymNodeDescriptionConversion;
use nym_api_requests::nym_nodes::NodeRole;
use std::time::Duration;
use time::OffsetDateTime;

/// The epoch transition is not instantaneous.
const EPOCH_TRANSITION_MARGIN: Duration = Duration::from_secs(120);

impl RetainedHeight {
    /// When the first snapshot taken after the epoch is due to change becomes served, estimated
    /// from this height alone.
    pub(crate) fn active_set_expiry(&self, settle_lag: usize) -> OffsetDateTime {
        let block_time = self.block_time();
        let due = self.mixnet().current_interval.current_epoch_end() + EPOCH_TRANSITION_MARGIN;
        let snapshot_interval = self.reads.snapshot_interval;
        let between_snapshots = AVERAGE_BLOCK_TIME * snapshot_interval;

        // the first snapshot whose estimated time is not before `due`, at least the next one
        let snapshots_ahead = match Duration::try_from(due - block_time) {
            Ok(until_due) => until_due
                .as_millis()
                .div_ceil(between_snapshots.as_millis())
                .max(1) as u32,
            Err(_) => 1,
        };
        block_time + AVERAGE_BLOCK_TIME * (snapshots_ahead * snapshot_interval + settle_lag as u32)
    }
}

/// The nodes of `subset` listed at the snapshot's height, with their roles, keys and performance
/// at that height.
pub(crate) fn build_nym_nodes_response<O>(
    snapshot: &RetainedHeight,
    subset: NodeSubset,
    active_only: bool,
) -> Vec<O>
where
    O: NymNodeDescriptionConversion,
{
    let rewarded_set = &snapshot.mixnet().rewarded_set;
    let current_key_rotation = snapshot.mixnet().current_key_rotation_id();

    let mut nodes = Vec::new();
    for nym_node in snapshot.node_view.nodes(subset) {
        let node_id = nym_node.node_id;

        let role: NodeRole = rewarded_set.role(node_id).into();

        // if the role is inactive, see if our filter allows it
        if active_only && role.is_inactive() {
            continue;
        }

        if let Some(node) = O::from_description(
            nym_node,
            current_key_rotation,
            role,
            snapshot.published_performance(node_id),
        ) {
            nodes.push(node);
        };
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::attested_snapshots::data::test_helpers::empty_height_reads;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Timestamp;
    use nym_mixnet_contract_common::Interval;

    const SETTLE_LAG: usize = 10;

    #[test]
    fn mid_epoch_the_active_set_expires_once_the_first_snapshot_after_the_epoch_is_due_is_served() {
        // due at 12:32:00, 1920 s ahead: the 4th snapshot (4 x 568.7 s) is the first not before it
        let height = taken_at_noon_with_epoch_ending(noon() + Duration::from_secs(30 * 60));

        // 12:38:51.67
        assert_eq!(
            height.active_set_expiry(SETTLE_LAG),
            noon() + Duration::from_millis(2_331_670)
        );
    }

    #[test]
    fn an_overdue_active_set_expires_once_the_next_snapshot_is_served() {
        let height = taken_at_noon_with_epoch_ending(noon() - Duration::from_secs(5 * 60));

        // 12:10:25.57
        assert_eq!(
            height.active_set_expiry(SETTLE_LAG),
            noon() + Duration::from_millis(625_570)
        );
    }

    /// 2026-01-01 12:00:00 UTC.
    fn noon() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_767_268_800).unwrap()
    }

    /// A height on a 100-block grid, taken at noon, in an epoch ending at `epoch_end`.
    fn taken_at_noon_with_epoch_ending(epoch_end: OffsetDateTime) -> RetainedHeight {
        let epoch_length = Duration::from_secs(60 * 60);
        let mut env = mock_env();
        env.block.time =
            Timestamp::from_seconds((epoch_end - epoch_length).unix_timestamp() as u64);

        let mut reads = empty_height_reads(1000);
        reads.block_time = noon();
        reads.snapshot_interval = 100;
        reads.mixnet.current_interval = Interval::init_interval(24, epoch_length, &env);
        RetainedHeight::new(reads, false)
    }
}
