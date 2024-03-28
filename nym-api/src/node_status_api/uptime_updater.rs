// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::{
    GatewayStatusReport, MixnodeStatusReport, NymApiStorageError,
};
use crate::node_status_api::ONE_DAY;
use crate::storage::NymApiStorage;
use nym_task::{TaskClient, TaskManager};
use std::time::Duration;
use time::{OffsetDateTime, PrimitiveDateTime, Time};
use tokio::time::{interval, sleep};
use tracing::error;
use tracing::{info, trace, warn};

pub(crate) struct HistoricalUptimeUpdater {
    storage: NymApiStorage,
}

impl HistoricalUptimeUpdater {
    pub(crate) fn new(storage: NymApiStorage) -> Self {
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
    ) -> Result<(Vec<MixnodeStatusReport>, Vec<GatewayStatusReport>), NymApiStorageError> {
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

    async fn update_uptimes(&self) -> Result<(), NymApiStorageError> {
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

    pub(crate) async fn run(&self, mut shutdown: TaskClient) {
        // update uptimes at 23:00 UTC each day so that we'd have data from the actual [almost] whole day
        // and so that we would avoid the edge case of starting validator API at 23:59 and having some
        // nodes update for different days

        // the unwrap is fine as 23:00:00 is a valid time
        let update_time = Time::from_hms(23, 0, 0).unwrap();
        let now = OffsetDateTime::now_utc();
        // is the current time within 0:00 - 22:59:59 or 23:00 - 23:59:59 ?
        let update_date = if now.hour() < 23 {
            now.date()
        } else {
            // the unwrap is fine as (**PRESUMABLY**) we're not running this code in the year 9999
            now.date().next_day().unwrap()
        };
        let update_datetime = PrimitiveDateTime::new(update_date, update_time).assume_utc();
        // the unwrap here is fine as we're certain `update_datetime` is in the future and thus the
        // resultant Duration is positive
        let time_left: Duration = (update_datetime - now).try_into().unwrap();

        info!(
            "waiting until {update_datetime} to update the historical uptimes for the first time ({} seconds left)", time_left.as_secs()
        );

        tokio::select! {
            biased;
            _ = shutdown.recv() => {
                trace!("UpdateHandler: Received shutdown");
            }
            _ = sleep(time_left) => {}
        }

        let mut interval = interval(ONE_DAY);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    trace!("UpdateHandler: Received shutdown");
                }
                _ = interval.tick() => {
                    // we don't want to have another select here; uptime update is relatively speedy
                    // and we don't want to exit while we're in the middle of database update
                    if let Err(err) = self.update_uptimes().await {
                        error!("We failed to update daily uptimes of active nodes - {err}");
                    }
                }
            }
        }
    }

    pub(crate) fn start(storage: NymApiStorage, shutdown: &TaskManager) {
        let uptime_updater = HistoricalUptimeUpdater::new(storage);
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { uptime_updater.run(shutdown_listener).await });
    }
}
