// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config;
use crate::error::NymRewarderError;
use crate::rewarder::epoch::Epoch;
use crate::rewarder::nyxd_client::NyxdClient;
use crate::rewarder::storage::RewarderStorage;
use crate::rewarder::ticketbook_issuance::monitor::CredentialIssuanceMonitor;
use crate::rewarder::ticketbook_issuance::types::{MonitoringResults, TicketbookIssuanceResults};
use nym_task::TaskClient;
use nym_validator_client::nyxd::AccountId;
use time::Date;
use tracing::info;

pub mod helpers;
mod monitor;
pub mod types;

pub struct TicketbookIssuance {
    pub(crate) nyxd_client: NyxdClient,

    // monitoring_results: MonitoringResults,
    pub(crate) storage: RewarderStorage,
}

impl TicketbookIssuance {
    pub(crate) async fn new(
        epoch: Epoch,
        storage: RewarderStorage,
        nyxd_client: &NyxdClient,
        whitelist: &[AccountId],
    ) -> Result<Self, NymRewarderError> {
        todo!()
        // Ok(TicketbookIssuance {
        //     // monitoring_results: MonitoringResults::new_initial(epoch, nyxd_client, whitelist)
        //     //     .await?,
        //     storage,
        // })
    }

    // no more background monitoring
    #[deprecated]
    pub(crate) fn start_monitor(
        &self,
        monitor_config: config::TicketbookIssuance,
        nyxd_client: NyxdClient,
        mut task_client: TaskClient,
    ) {
        task_client.disarm();

        // let monitoring_results = self.monitoring_results.clone();
        // let mut monitor = CredentialIssuanceMonitor::new(
        //     monitor_config,
        //     nyxd_client,
        //     self.storage.clone(),
        //     monitoring_results,
        // );
        //
        // tokio::spawn(async move { monitor.run(task_client).await });
    }

    pub(crate) async fn get_issued_credentials_results(
        &self,
        current_epoch: Epoch,
    ) -> Result<TicketbookIssuanceResults, NymRewarderError> {
        todo!()
        // info!(
        //     "looking up credential issuers for epoch {} ({} - {})",
        //     current_epoch.id,
        //     current_epoch.start_rfc3339(),
        //     current_epoch.end_rfc3339()
        // );
        //
        // let raw_results = self.monitoring_results.finish_epoch().await;
        //
        // Ok(raw_results.into())
    }

    pub(crate) async fn get_issued_ticketbooks_results(
        &self,
        date: Date,
    ) -> Result<TicketbookIssuanceResults, NymRewarderError> {
        info!("checking for all issued ticketbooks on {date}");
        // let ecash_contract = self.nyxd_client.
        todo!()
    }
}
