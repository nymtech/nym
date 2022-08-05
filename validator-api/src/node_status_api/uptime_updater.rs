// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node_status_api::models::{
    GatewayStatusReport, MixnodeStatusReport, ValidatorApiStorageError,
};
use crate::node_status_api::ONE_DAY;
use crate::storage::ValidatorApiStorage;
use log::error;
use task::ShutdownListener;
use time::OffsetDateTime;
use tokio::time::sleep;

pub(crate) struct HistoricalUptimeUpdater {
    storage: ValidatorApiStorage,
}

impl HistoricalUptimeUpdater {
    pub(crate) fn new(storage: ValidatorApiStorage) -> Self {
        HistoricalUptimeUpdater { storage }
    }

    /// Obtains the lists of all mixnodes and gateways that were tested at least a single time
    /// in the last 24h interval.
    ///
    /// # Arguments
    ///
    /// * `now`: current time.
    async fn get_active_nodes(
        &self,
        now: OffsetDateTime,
    ) -> Result<(Vec<MixnodeStatusReport>, Vec<GatewayStatusReport>), ValidatorApiStorageError>
    {
        let day_ago = (now - ONE_DAY).unix_timestamp();
        let active_mixnodes = self
            .storage
            .get_all_active_mixnode_reports_in_interval(day_ago, now.unix_timestamp())
            .await?;

        let active_gateways = self
            .storage
            .get_all_active_gateway_reports_in_interval(day_ago, now.unix_timestamp())
            .await?;

        Ok((active_mixnodes, active_gateways))
    }

    async fn update_uptimes(&self) -> Result<(), ValidatorApiStorageError> {
        let now = OffsetDateTime::now_utc();
        let today_iso_8601 = now.date().to_string();

        // get nodes that were active in last 24h
        let (active_mixnodes, active_gateways) = self.get_active_nodes(now).await?;

        if self
            .storage
            .check_if_historical_uptimes_exist_for_date(&today_iso_8601)
            .await?
        {
            warn!("We have already updated uptimes for all nodes this day.")
        } else {
            info!("Updating historical daily uptimes of all nodes...");
            self.storage
                .update_historical_uptimes(&today_iso_8601, &active_mixnodes, &active_gateways)
                .await?;
        }

        Ok(())
    }

    pub(crate) async fn run(&self, mut shutdown: ShutdownListener) {
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = sleep(ONE_DAY) => {
                    if let Err(err) = self.update_uptimes().await {
                        // normally that would have been a warning rather than an error,
                        // however, in this case it implies some underlying issues with our database
                        // that might affect the entire program
                        error!(
                            "We failed to update daily uptimes of active nodes - {}",
                            err
                        )
                    }
                }
                _ = shutdown.recv() => {
                    trace!("UpdateHandler: Received shutdown");
                }
            }
        }
    }
}
