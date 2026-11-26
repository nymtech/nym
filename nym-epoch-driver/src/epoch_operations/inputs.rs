// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The two epoch-time reads rewarding and selection take their inputs from. A failure of either
//! fails the epoch operation, which is retried.

use crate::epoch_operations::EpochAdvancer;
use crate::network_view::NetworkView;
use anyhow::Context;
use nym_directory_client::cadence::SnapshotCadence;
use nym_mixnet_contract_common::EpochId;
use tracing::{error, info};

impl EpochAdvancer {
    /// The network view selection reads, at the newest settled cadence boundary.
    pub(super) async fn produce_network_view(&self) -> anyhow::Result<NetworkView> {
        info!("Producing the network view at the newest settled cadence boundary...");
        match self._produce_network_view().await {
            Ok(network_view) => {
                info!("Produced the network view... SUCCESS");
                Ok(network_view)
            }
            Err(err) => {
                error!("FAILED to produce the network view: {err:#}");
                Err(err)
            }
        }
    }

    async fn _produce_network_view(&self) -> anyhow::Result<NetworkView> {
        let cadence = SnapshotCadence::read(&self.nyxd_client)
            .await
            .context("failed to read the snapshot cadence")?;
        let height = cadence
            .newest_settled_height(self.config.settle_lag)
            .context("no cadence boundary has settled to produce the network view at")?;

        // essentially get all bonded nodes, their stake, their available roles, etc.
        // from the last settled height
        NetworkView::produce(&self.directory_anchor, &self.nyxd_client, height).await
    }

    /// Brings the window up to `epoch_id`, whose transition has begun, so that nothing more can
    /// land for it.
    // note that this replicates a bit of what the contract does, or rather will do,
    // once mixnet contract gets performance directly from the contract instead
    pub(super) async fn update_performance_window(
        &mut self,
        epoch_id: EpochId,
    ) -> anyhow::Result<()> {
        info!("Bringing the performance contract window up to epoch {epoch_id}...");
        match self.window.update(&self.nyxd_client, epoch_id).await {
            Ok(()) => {
                info!("Brought the performance contract window up to date... SUCCESS");
                Ok(())
            }
            Err(err) => {
                error!("FAILED to bring the performance contract window up to date: {err:#}");
                Err(err)
            }
        }
    }
}
