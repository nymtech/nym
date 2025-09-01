// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_gateway_storage::{GatewayStorage, InboxManager};
use nym_task::ShutdownToken;
use std::error::Error;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::task::JoinHandle;
use tracing::{debug, trace, warn};

pub struct StaleMessagesCleaner {
    inbox_manager: InboxManager,
    shutdown_token: ShutdownToken,
    max_message_age: Duration,
    run_interval: Duration,
}

impl StaleMessagesCleaner {
    pub(crate) fn new(
        storage: &GatewayStorage,
        shutdown_token: ShutdownToken,
        max_message_age: Duration,
        run_interval: Duration,
    ) -> Self {
        StaleMessagesCleaner {
            inbox_manager: storage.inbox_manager().clone(),
            shutdown_token,
            max_message_age,
            run_interval,
        }
    }

    async fn clean_up_stale_messages(&mut self) -> Result<(), impl Error> {
        let cutoff = OffsetDateTime::now_utc() - self.max_message_age;
        self.inbox_manager.remove_stale(cutoff).await
    }

    async fn run(&mut self) {
        let mut interval = tokio::time::interval(self.run_interval);
        while !self.shutdown_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("StaleMessagesCleaner: received shutdown");
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

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
