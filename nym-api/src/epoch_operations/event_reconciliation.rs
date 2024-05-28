// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::error::RewardingError;
use crate::RewardedSetUpdater;
use log::{error, warn};
use nym_mixnet_contract_common::EpochState;
use std::cmp::max;

impl RewardedSetUpdater {
    pub(super) async fn reconcile_epoch_events(&self) -> Result<(), RewardingError> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        match epoch_status.state {
            state @ EpochState::InProgress | state @ EpochState::Rewarding { .. } => {
                // hard error, this shouldn't have happened!
                error!("tried to perform node rewarding while in {state} state!");
                Err(RewardingError::InvalidEpochState {
                    current_state: state,
                    operation: "reconciling epoch events".to_string(),
                })
            }
            EpochState::AdvancingEpoch => {
                warn!("we seem to have crashed mid epoch operations... no need to reconcile events as we've already done that!");
                Ok(())
            }
            EpochState::ReconcilingEvents => {
                log::info!("Reconciling all pending epoch events...");
                if let Err(err) = self._reconcile_epoch_events().await {
                    log::error!("FAILED to reconcile epoch events... - {err}");
                    Err(err)
                } else {
                    log::info!("Reconciled all pending epoch events... SUCCESS");
                    Ok(())
                }
            }
        }
    }

    async fn _reconcile_epoch_events(&self) -> Result<(), RewardingError> {
        let pending_events = self.nyxd_client.get_pending_events_count().await?;

        // be conservative about number of events we resolve at once.
        // contract should be able to handle few hundred at once
        // but keep it on the lower side.
        let limit = 100;

        let mut required_calls = pending_events / limit;
        if pending_events % limit != 0 {
            required_calls += 1;
        }

        // make sure to at least call it once so that if there are no pending events,
        // the epoch would get advanced into the next phase
        for _ in 0..max(required_calls, 1) {
            self.nyxd_client.reconcile_epoch_events(Some(limit)).await?;
        }

        // in the incredibly unlikely/borderline impossible scenario a HUGE number of events got pushed
        // while we were reconciling the events, do it one more time
        //
        // note: it's perfectly fine if we don't clear EXACTLY everything,
        // since when we execute transaction to actually advance the epoch,
        // it will resolve all remaining events.
        let pending_events = self.nyxd_client.get_pending_events_count().await?;
        if pending_events > 20 {
            self.nyxd_client.reconcile_epoch_events(Some(limit)).await?;
        }

        Ok(())
    }
}
