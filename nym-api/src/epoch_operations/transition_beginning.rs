// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::error::RewardingError;
use crate::epoch_operations::RewardedSetUpdater;

impl RewardedSetUpdater {
    // returns boolean indicating whether we should bother continuing
    pub(super) async fn begin_epoch_transition(&self) -> Result<bool, RewardingError> {
        info!("starting the epoch transition...");
        if let Err(err) = self._begin_epoch_transition().await {
            // perform the state query again to make sure it's not because other nym-api (not yet applicable)
            // wasn't faster than us
            let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
            if !epoch_status.is_in_progress() {
                log::error!("FAILED to begin epoch progression: {err}");
                Err(err)
            } else {
                error!("another nym-api ({}) is already advancing the epoch... but we shouldn't have other nym-apis yet!", epoch_status.being_advanced_by);
                Ok(false)
            }
        } else {
            log::info!("Begun epoch transition... SUCCESS");
            Ok(true)
        }
    }

    async fn _begin_epoch_transition(&self) -> Result<(), RewardingError> {
        Ok(self.nyxd_client.begin_epoch_transition().await?)
    }
}
