// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The loop that keeps this service's in-memory snapshots of on-chain node data current.
//!
//! One tick selects one cadence height and refreshes every snapshot at it. That is the point of
//! the shared grid: the interval belongs to the directory contract and every attested contract
//! reads on the same boundaries, so one selection per tick is what keeps those reads joinable
//! rather than merely recent. Geolocation is the only snapshot so far; a directory read joins
//! the same tick at the same height.
//!
//! Deliberately not a step of the monitor cycle: no cycle step reads or writes these snapshots,
//! so putting them there would couple a chain read to a sequence of database writes while
//! establishing no ordering anything relies on. Kept apart, a failing monitor cycle leaves the
//! snapshots refreshing, and a slow chain read delays nothing the monitor writes.

use crate::directory::select_cadence_height;
use crate::geolocation::GeoSnapshotHandle;
use crate::geolocation::refresh::GeolocationRefresher;
use futures_util::StreamExt;
use nym_task::ShutdownToken;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::time::Duration;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use tracing::{error, trace};

pub(crate) struct NodeDataRefreshWorker {
    /// Held for the cadence height. Separate from any reader's own client because selecting the
    /// height is a different job from reading at it, and against a different contract.
    nyx_client: QueryHttpRpcNyxdClient,
    geolocation: GeolocationRefresher<QueryHttpRpcNyxdClient>,
    refresh_interval: Duration,
    shutdown_token: ShutdownToken,
}

impl NodeDataRefreshWorker {
    pub(crate) fn new(
        nyx_client: &QueryHttpRpcNyxdClient,
        geo_snapshot: GeoSnapshotHandle,
        refresh_interval: Duration,
        shutdown_token: ShutdownToken,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            nyx_client: nyx_client.clone_query_client(),
            geolocation: GeolocationRefresher::new(nyx_client, geo_snapshot)?,
            refresh_interval,
            shutdown_token,
        })
    }

    /// Refresh on the interval, starting immediately, since the first tick fires at once and the
    /// snapshots are empty until it lands.
    ///
    /// A failure is logged and waits for the next tick like any other outcome. There is no
    /// sooner retry: the held snapshot keeps being served meanwhile, and a failure a retry
    /// would fix is fixed by the next interval just as well.
    pub(crate) async fn run(self) {
        let mut refreshes = IntervalStream::new(interval(self.refresh_interval));

        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("NodeDataRefreshWorker: Received shutdown");
                    break;
                }
                _ = refreshes.next() => {
                    if let Err(err) = self.refresh_at_cadence_height().await {
                        error!("node data refresh failed: {err:#}");
                    }
                }
            }
        }
    }

    /// Select the height once, then read every snapshot at it.
    async fn refresh_at_cadence_height(&self) -> anyhow::Result<()> {
        let height = select_cadence_height(&self.nyx_client).await?;

        self.geolocation.refresh(height).await
    }
}
