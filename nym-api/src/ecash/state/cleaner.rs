// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::storage::EcashStorageExt;
use crate::node_status_api::models::NymApiStorageError;
use crate::support::config::Config;
use crate::support::storage::NymApiStorage;
use nym_ecash_time::ecash_today_date;
use nym_task::TaskClient;
use std::time::Duration;
use time::Date;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace};

/// Task responsible for clearing out the database from stale ecash data,
/// such as verified tickets or issued partial ticketbooks.
pub struct EcashBackgroundStateCleaner {
    run_interval: Duration,
    issued_ticketbooks_retention_period_days: u32,
    verified_tickets_retention_period_days: u32,

    storage: NymApiStorage,
    task_client: TaskClient,
}

impl EcashBackgroundStateCleaner {
    pub fn new(global_config: &Config, storage: NymApiStorage, task_client: TaskClient) -> Self {
        EcashBackgroundStateCleaner {
            run_interval: global_config.ecash_signer.debug.stale_data_cleaner_interval,
            issued_ticketbooks_retention_period_days: global_config
                .ecash_signer
                .debug
                .issued_ticketbooks_retention_period_days,
            verified_tickets_retention_period_days: global_config
                .ecash_signer
                .debug
                .verified_tickets_retention_period_days,
            storage,
            task_client,
        }
    }

    fn ticketbook_retention_cutoff(&self) -> Date {
        ecash_today_date()
            - time::Duration::days(self.issued_ticketbooks_retention_period_days as i64)
    }

    fn verified_tickets_retention_cutoff(&self) -> Date {
        ecash_today_date()
            - time::Duration::days(self.verified_tickets_retention_period_days as i64)
    }

    async fn clean_stale_data(&self) -> Result<(), NymApiStorageError> {
        // 1. remove old verified tickets
        self.storage
            .remove_expired_verified_tickets(self.verified_tickets_retention_cutoff())
            .await?;

        // 2. remove old issued partial ticketbooks
        self.storage
            .remove_old_issued_ticketbooks(self.ticketbook_retention_cutoff())
            .await?;

        Ok(())
    }

    async fn run(&mut self) {
        let mut ticker = tokio::time::interval(self.run_interval);
        loop {
            tokio::select! {
                _ = self.task_client.recv() => {
                    trace!("EcashBackgroundStateCleaner: Received shutdown");
                    break;
                }
                _ = ticker.tick() => {
                    if let Err(err) = self.clean_stale_data().await {
                        error!("failed to clear out stale data: {err}")
                    }
                }
            }
        }

        debug!("EcashBackgroundStateCleaner: exiting");
    }

    pub(crate) fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
