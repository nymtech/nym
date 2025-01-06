// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_gateway_storage::{GatewayStorage, InboxManager};
use nym_task::TaskClient;
use std::error::Error;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::task::JoinHandle;
use tracing::{trace, warn};

pub struct StaleMessagesCleaner {
    inbox_manager: InboxManager,
    task_client: TaskClient,
    max_message_age: Duration,
    run_interval: Duration,
}

impl StaleMessagesCleaner {
    pub(crate) fn new(
        storage: &GatewayStorage,
        task_client: TaskClient,
        max_message_age: Duration,
        run_interval: Duration,
    ) -> Self {
        StaleMessagesCleaner {
            inbox_manager: storage.inbox_manager().clone(),
            task_client,
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
        while !self.task_client.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.task_client.recv() => {
                    trace!("StaleMessagesCleaner: received shutdown");
                }
                _ = interval.tick() => {
                    if let Err(err) = self.clean_up_stale_messages().await {
                        warn!("failed to clean up stale messages: {err}");
                    }
                }
            }
        }
    }

    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
