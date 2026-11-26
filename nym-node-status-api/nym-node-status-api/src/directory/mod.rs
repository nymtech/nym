// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What this service takes from the directory contract.
//!
//! Only the shared cadence grid so far, and it is here rather than beside the geolocation read
//! that currently uses it because the interval is the directory contract's: the cadence is
//! specified as shared across every attested contract, so one height serves all of them. A
//! later directory read pins to the same grid this selects, which is half the reason the
//! geolocation read pins to it now.

use anyhow::{Context, anyhow};
use nym_directory_client::cadence::SnapshotCadence;
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

/// Read the interval and the tip, and place this refresh on the grid they describe.
pub(crate) async fn select_cadence_height<C>(client: &C) -> anyhow::Result<Height>
where
    C: DirectoryQueryClient + TendermintRpcClient + Sync,
{
    let cadence = SnapshotCadence::read(client)
        .await
        .context("failed to read the snapshot cadence")?;

    cadence.newest_settled_height(SETTLE_LAG).ok_or_else(|| {
        anyhow!(
            "no cadence height exists at tip {} for a snapshot interval of {}",
            cadence.tip,
            cadence.interval.interval
        )
    })
}
