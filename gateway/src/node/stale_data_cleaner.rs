// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_gateway_storage::{GatewayStorage, InboxManager};
use nym_task::ShutdownToken;
use std::error::Error;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{debug, trace, warn};

pub struct StaleMessagesCleaner {
    inbox_manager: InboxManager,
    max_message_age: Duration,
    run_interval: Duration,
}

impl StaleMessagesCleaner {
    pub(crate) fn new(
        storage: &GatewayStorage,
        max_message_age: Duration,
        run_interval: Duration,
    ) -> Self {
        StaleMessagesCleaner {
            inbox_manager: storage.inbox_manager().clone(),
            max_message_age,
            run_interval,
        }
    }

    async fn clean_up_stale_messages(&mut self) -> Result<(), impl Error> {
        let cutoff = OffsetDateTime::now_utc() - self.max_message_age;
        self.inbox_manager.remove_stale(cutoff).await
    }

    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        let mut interval = tokio::time::interval(self.run_interval);
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("StaleMessagesCleaner: received shutdown");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(err) = self.clean_up_stale_messages().await {
                        warn!("failed to clean up stale messages: {err}");
                    }
                }
            }
        }
        debug!("StaleMessagesCleaner: Exiting");
    }
}
