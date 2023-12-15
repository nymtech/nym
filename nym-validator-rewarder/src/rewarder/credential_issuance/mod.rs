// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::credential_issuance::monitor::CredentialIssuanceMonitor;
use crate::rewarder::credential_issuance::types::{CredentialIssuanceResults, MonitoringResults};
use crate::rewarder::epoch::Epoch;
use nym_task::TaskClient;
use nym_validator_client::nyxd::AccountId;
use std::time::Duration;
use tracing::info;

mod monitor;
pub mod types;

pub struct CredentialIssuance {
    monitoring_run_interval: Duration,
    monitoring_results: MonitoringResults,
    dkg_contract_address: AccountId,
}

impl CredentialIssuance {
    pub(crate) fn new(
        epoch: Epoch,
        monitoring_run_interval: Duration,
        dkg_contract_address: AccountId,
    ) -> Self {
        CredentialIssuance {
            monitoring_run_interval,
            monitoring_results: MonitoringResults::new(epoch),
            dkg_contract_address,
        }
    }

    pub(crate) fn start_monitor(&self, task_client: TaskClient) {
        let monitoring_results = self.monitoring_results.clone();
        let mut monitor = CredentialIssuanceMonitor::new(
            self.monitoring_run_interval,
            self.dkg_contract_address.clone(),
            monitoring_results,
        );

        tokio::spawn(async move { monitor.run(task_client).await });
    }

    pub(crate) async fn get_issued_credentials_results(
        &self,
        current_epoch: Epoch,
    ) -> Result<CredentialIssuanceResults, NymRewarderError> {
        info!(
            "looking up credential issuers for epoch {} ({} - {})",
            current_epoch.id,
            current_epoch.start_rfc3339(),
            current_epoch.end_rfc3339()
        );

        let raw_results = self.monitoring_results.finish_epoch().await;

        Ok(raw_results.into())
    }
}
