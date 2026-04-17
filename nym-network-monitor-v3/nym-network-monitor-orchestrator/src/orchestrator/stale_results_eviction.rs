// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::NetworkMonitorStorage;
use std::time::Duration;
use tokio::time::{Instant, interval_at};
use tracing::error;

pub(crate) struct StaleResultsEviction {
    storage: NetworkMonitorStorage,
    max_result_age: Duration,
    max_testrun_timeout: Duration,
    check_interval: Duration,
}

impl StaleResultsEviction {
    pub(crate) fn new(
        storage: NetworkMonitorStorage,
        max_result_age: Duration,
        max_testrun_timeout: Duration,
    ) -> Self {
        // let check interval be half of the minimum of the two timeouts
        let check_interval = Duration::min(max_result_age, max_testrun_timeout) / 2;

        Self {
            storage,
            max_result_age,
            max_testrun_timeout,
            check_interval,
        }
    }

    pub(crate) async fn evict_stale_results(&self) -> anyhow::Result<()> {
        self.storage
            .clear_timed_out_testruns_in_progress(self.max_testrun_timeout)
            .await?;
        self.storage.evict_old_testruns(self.max_result_age).await?;
        Ok(())
    }

    pub(crate) async fn run(&self) {
        let mut interval = interval_at(Instant::now() + self.check_interval, self.check_interval);
        loop {
            interval.tick().await;
            if let Err(err) = self.evict_stale_results().await {
                error!("Failed to evict stale results: {err}");
            }
        }
    }
}
