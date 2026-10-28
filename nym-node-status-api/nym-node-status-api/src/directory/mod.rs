// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What this service takes from the directory contract.
//!
//! Only the shared cadence grid so far, and it is here rather than beside the geolocation read
//! that currently uses it because the interval is the directory contract's: the cadence is
//! specified as shared across every attested contract, so one height serves all of them. A
//! later directory read pins to the same grid this computes, which is half the reason the
//! geolocation read pins to it now.

use anyhow::{Context, anyhow};
use nym_validator_client::nyxd::contract_traits::DirectoryQueryClient;
use nym_validator_client::nyxd::{Height, TendermintRpcClient};

/// How far behind the tip a readable height sits.
///
/// One block is the hard requirement: a digest proof at `H` verifies against the `app_hash`
/// carried in the header at `H+1`, so the tip itself is never readable. The second absorbs an
/// RPC answering from a block it has not caught up with yet. It stays this small because every
/// block added here is a block added to the deployment requirement that the RPC retain at least
/// `interval + SETTLE_LAG` blocks of state, and a nyx signer RPC has been observed retaining
/// barely a hundred.
const SETTLE_LAG: u64 = 2;

/// The greatest multiple of `interval` at or below `tip - SETTLE_LAG`.
///
/// `None` when no such height exists yet, which covers a chain that has not completed an
/// interval behind the lag, and an interval of zero, which describes no grid at all.
pub(crate) fn cadence_height(tip: Height, interval: u32) -> Option<Height> {
    let interval = u64::from(interval);
    if interval == 0 {
        return None;
    }

    let settled = tip.value().checked_sub(SETTLE_LAG)?;
    let boundary = settled - (settled % interval);
    if boundary == 0 {
        return None;
    }

    Height::try_from(boundary).ok()
}

/// Read the interval and the tip, and place this refresh on the grid they describe.
///
/// The interval is re-read on every call rather than cached at start-up: it is mutable on chain,
/// and a stale copy would quietly put this service on a different grid from every other consumer
/// of the same cadence.
pub(crate) async fn select_cadence_height<C>(client: &C) -> anyhow::Result<Height>
where
    C: DirectoryQueryClient + TendermintRpcClient + Sync,
{
    let interval = client
        .get_snapshot_interval()
        .await
        .context("failed to read the snapshot interval from the directory contract")?
        .interval;

    let tip = client
        .latest_block()
        .await
        .context("failed to read the chain tip")?
        .block
        .header
        .height;

    cadence_height(tip, interval).ok_or_else(|| {
        anyhow!("no cadence height exists at tip {tip} for a snapshot interval of {interval}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: u32 = 100;

    fn height(value: u64) -> Height {
        Height::try_from(value).expect("test height")
    }

    /// The failure this guards against is silent: an off-grid height still reads, still proves
    /// and still serves, it just puts this service on a grid of its own.
    #[test]
    fn every_selected_height_is_on_the_grid_and_is_the_newest_one_available() {
        for tip in [1_000u64, 1_001, 1_002, 1_099, 1_101, 9_999] {
            let selected = cadence_height(height(tip), INTERVAL)
                .expect("a chain this long has a cadence height")
                .value();

            assert_eq!(
                selected % u64::from(INTERVAL),
                0,
                "tip {tip} selected off-grid height {selected}"
            );
            assert!(
                selected <= tip - SETTLE_LAG,
                "tip {tip} selected {selected}, which the lag has not settled"
            );
            assert!(
                tip - selected < u64::from(INTERVAL) + SETTLE_LAG,
                "tip {tip} selected {selected}, skipping a newer boundary"
            );
        }
    }

    /// The lag's whole effect, at the only point it is observable.
    #[test]
    fn a_boundary_becomes_readable_once_the_lag_has_passed() {
        assert_eq!(cadence_height(height(1_001), INTERVAL), Some(height(900)));
        assert_eq!(cadence_height(height(1_002), INTERVAL), Some(height(1_000)));
    }

    #[test]
    fn no_height_where_there_is_no_grid_to_land_on() {
        // the chain has not completed an interval behind the lag
        assert_eq!(cadence_height(height(99), INTERVAL), None);
        assert_eq!(cadence_height(height(101), INTERVAL), None);
        // and would underflow computing the lag at all
        assert_eq!(cadence_height(height(1), INTERVAL), None);
        // an interval of zero describes no grid, rather than dividing by it
        assert_eq!(cadence_height(height(1_000), 0), None);
    }
}
