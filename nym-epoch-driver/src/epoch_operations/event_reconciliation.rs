// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use crate::prometheus::PrometheusMetric;
use anyhow::bail;
use nym_mixnet_contract_common::EpochState;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, MixnetSigningClient};
use tracing::{error, info, warn};

/// How many events one reconciliation resolves: the contract handles a few hundred at once, but
/// keep it on the lower side.
const RECONCILIATION_BATCH: u32 = 100;

impl EpochAdvancer {
    pub(super) async fn reconcile_epoch_events(&self) -> anyhow::Result<()> {
        let epoch_status = self.nyxd_client.get_current_epoch_status().await?;
        match epoch_status.state {
            state @ EpochState::InProgress | state @ EpochState::Rewarding { .. } => {
                // hard error, this shouldn't have happened!
                error!("tried to perform node rewarding while in {state} state!");
                bail!(
                    "the current epoch is in the wrong state ({state}) to perform the requested operation: reconciling epoch events"
                )
            }
            EpochState::RoleAssignment { .. } => {
                warn!(
                    "we seem to have crashed mid epoch operations... no need to reconcile events as we've already done that!"
                );
                Ok(())
            }
            EpochState::ReconcilingEvents => {
                info!("Reconciling all pending epoch events...");
                if let Err(err) = self._reconcile_epoch_events().await {
                    error!("FAILED to reconcile epoch events... - {err}");
                    Err(err)
                } else {
                    info!("Reconciled all pending epoch events... SUCCESS");
                    Ok(())
                }
            }
        }
    }

    async fn _reconcile_epoch_events(&self) -> anyhow::Result<()> {
        let pending = self.nyxd_client.get_number_of_pending_events().await?;
        PrometheusMetric::PendingEpochEvents.set(pending.epoch_events.into());
        PrometheusMetric::PendingIntervalEvents.set(pending.interval_events.into());

        // the contract moves on to role assignment once no epoch events are left, nor interval
        // events once the interval is over; at least one call, so that it does with none pending
        loop {
            self.nyxd_client
                .reconcile_epoch_events(Some(RECONCILIATION_BATCH), None)
                .await?;
            let state = self.nyxd_client.get_current_epoch_status().await?.state;
            if !matches!(state, EpochState::ReconcilingEvents) {
                return Ok(());
            }
        }
    }
}
