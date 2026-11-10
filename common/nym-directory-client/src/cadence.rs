// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The directory contract's snapshot cadence: the grid of heights every attested contract is
//! snapshotted and read at, so that independent readers land on the same heights.

use nym_directory_contract_common::SnapshotIntervalResponse;
use nym_validator_client::nyxd::contract_traits::DirectoryQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{Height, TendermintRpcClient};

/// The snapshot interval and the chain tip, read together.
#[derive(Debug, Clone)]
pub struct SnapshotCadence {
    pub interval: SnapshotIntervalResponse,
    pub tip: Height,
}

impl SnapshotCadence {
    /// Reads the interval afresh rather than taking a cached copy: it is mutable on chain, and a
    /// stale one would put the reader on a grid of its own.
    pub async fn read<C>(client: &C) -> Result<Self, NyxdError>
    where
        C: DirectoryQueryClient + TendermintRpcClient + Sync,
    {
        let interval = client.get_snapshot_interval().await?;
        let tip = client.latest_block().await?.block.header.height;
        Ok(SnapshotCadence { interval, tip })
    }

    /// The newest cadence height at least `settle_lag` blocks below the tip.
    pub fn newest_settled_height(&self, settle_lag: u64) -> Option<Height> {
        self.settled_heights(settle_lag, 1).into_iter().next()
    }

    /// The newest `count` cadence heights at least `settle_lag` blocks below the tip, newest
    /// first, fewer on a young chain. The lag is at least one block, since a proof at `H`
    /// verifies against the app hash carried in the header at `H+1`.
    pub fn settled_heights(&self, settle_lag: u64, count: usize) -> Vec<Height> {
        let Some(newest) = self
            .tip
            .value()
            .checked_sub(settle_lag.max(1))
            .and_then(|settled| self.interval.boundary_at_or_below(settled))
        else {
            return Vec::new();
        };

        let interval = u64::from(self.interval.interval);
        (0..count as u64)
            .map(|i| newest.saturating_sub(interval * i))
            .take_while(|height| *height > 0)
            .filter_map(|height| Height::try_from(height).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cadence(interval: u32, tip: u64) -> SnapshotCadence {
        SnapshotCadence {
            interval: SnapshotIntervalResponse { interval },
            tip: height(tip),
        }
    }

    fn height(value: u64) -> Height {
        Height::try_from(value).unwrap()
    }

    fn heights(values: &[u64]) -> Vec<Height> {
        values.iter().map(|value| height(*value)).collect()
    }

    #[test]
    fn the_newest_settled_height_is_the_newest_boundary_the_lag_has_passed() {
        assert_eq!(
            cadence(100, 1_050).newest_settled_height(1),
            Some(height(1_000))
        );
        // one block past the boundary is enough: header[1001] carries the app hash for 1000
        assert_eq!(
            cadence(100, 1_001).newest_settled_height(1),
            Some(height(1_000))
        );
        assert_eq!(
            cadence(100, 1_001).newest_settled_height(2),
            Some(height(900))
        );
        assert_eq!(
            cadence(100, 1_002).newest_settled_height(2),
            Some(height(1_000))
        );
    }

    #[test]
    fn a_boundary_on_the_tip_is_never_settled_whatever_the_lag() {
        // the app hash committing state at H lives in header[H+1]
        assert_eq!(
            cadence(100, 1_000).newest_settled_height(0),
            Some(height(900))
        );
        assert_eq!(
            cadence(100, 1_000).newest_settled_height(1),
            Some(height(900))
        );
    }

    #[test]
    fn there_is_no_settled_height_without_a_grid_to_land_on() {
        // the chain has not completed an interval behind the lag
        assert_eq!(cadence(100, 100).newest_settled_height(1), None);
        assert_eq!(cadence(100, 101).newest_settled_height(2), None);
        // and would underflow computing the lag at all
        assert_eq!(cadence(100, 1).newest_settled_height(2), None);
        // an interval of zero describes no grid, rather than dividing by it
        assert_eq!(cadence(0, 1_000).newest_settled_height(1), None);
    }

    #[test]
    fn settled_heights_are_the_newest_boundaries_counting_back() {
        assert_eq!(
            cadence(100, 1_050).settled_heights(1, 3),
            heights(&[1_000, 900, 800])
        );
        assert_eq!(
            cadence(100, 1_000).settled_heights(1, 2),
            heights(&[900, 800])
        );
    }

    #[test]
    fn a_young_chain_settles_only_the_boundaries_it_has() {
        // height 0 is not a snapshot
        assert_eq!(cadence(100, 150).settled_heights(1, 4), heights(&[100]));
        assert_eq!(cadence(100, 50).settled_heights(1, 4), heights(&[]));
    }
}
