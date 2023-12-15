// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NymRewarderError;
use crate::rewarder::credential_issuance::types::MonitoringResults;
use nym_task::TaskClient;
use nym_validator_client::nyxd::AccountId;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info};

pub struct CredentialIssuanceMonitor {
    run_interval: Duration,
    dkg_contract_address: AccountId,
    monitoring_results: MonitoringResults,
}

impl CredentialIssuanceMonitor {
    pub fn new(
        run_interval: Duration,
        dkg_contract_address: AccountId,
        monitoring_results: MonitoringResults,
    ) -> CredentialIssuanceMonitor {
        CredentialIssuanceMonitor {
            run_interval,
            dkg_contract_address,
            monitoring_results,
        }
    }

    // 1. if not present -> go to DKG contract and grab the accounts + endpoints

    async fn check_issuers(&mut self) -> Result<(), NymRewarderError> {
        Ok(())
    }

    pub async fn run(&mut self, mut task_client: TaskClient) {
        info!("starting");
        let mut run_interval = interval(self.run_interval);

        while !task_client.is_shutdown() {
            tokio::select! {
                biased;
                _ = task_client.recv() => {
                    info!("received shutdown");
                    break
                }
                _ = run_interval.tick() => {
                    if let Err(err) = self.check_issuers().await {
                        error!("failed to perform credential issuance check: {err}")
                    }
                }
            }
        }
    }
}
