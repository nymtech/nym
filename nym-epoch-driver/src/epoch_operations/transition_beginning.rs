// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use anyhow::bail;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use tracing::{error, info, warn};

impl EpochAdvancer {
    /// Begins the epoch transition, or resumes the one this account began before a restart.
    pub(super) async fn begin_epoch_transition(&self) -> anyhow::Result<()> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        if !epoch_status.is_in_progress() {
            let advancing = epoch_status.being_advanced_by;
            if advancing.as_str() != self.nyxd_client.address().as_ref() {
                bail!("another account ({advancing}) is advancing the epoch");
            }
            warn!("resuming the epoch transition begun before a restart");
            return Ok(());
        }

        info!("Beginning the epoch transition...");
        match self.nyxd_client.begin_epoch_transition(None).await {
            Ok(_) => {
                info!("Began the epoch transition... SUCCESS");
                Ok(())
            }
            Err(err) => {
                error!("FAILED to begin the epoch transition: {err}");
                Err(err.into())
            }
        }
    }
}
