// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use tracing::{error, info};

impl EpochAdvancer {
    // returns boolean indicating whether we should bother continuing
    pub(super) async fn begin_epoch_transition(&self) -> anyhow::Result<bool> {
        info!("starting the epoch transition...");
        if let Err(err) = self._begin_epoch_transition().await {
            // perform the state query again to make sure it's not because another account
            // wasn't faster than us
            let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
            if !epoch_status.is_in_progress() {
                error!("FAILED to begin epoch progression: {err}");
                Err(err)
            } else {
                error!(
                    "another account ({}) is already advancing the epoch... but only the rewarding validator should!",
                    epoch_status.being_advanced_by
                );
                Ok(false)
            }
        } else {
            info!("Begun epoch transition... SUCCESS");
            Ok(true)
        }
    }

    async fn _begin_epoch_transition(&self) -> anyhow::Result<()> {
        self.nyxd_client.begin_epoch_transition(None).await?;
        Ok(())
    }
}
