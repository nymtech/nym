// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilter;
use nym_task::ShutdownToken;
use tracing::{info, trace, warn};

// background task responsible for periodically flushing the bloomfilter to disk
// as well as clearing it up on the specified timer
// (in the future this will be enforced by key rotation)
pub struct ReplayProtectionBackgroundTask {
    filter: ReplayProtectionBloomfilter,
    shutdown_token: ShutdownToken,
}

impl ReplayProtectionBackgroundTask {
    pub(crate) fn new(shutdown_token: ShutdownToken) -> Self {
        todo!()
        // Self { shutdown_token }
    }

    async fn flush_to_disk(&self) -> Result<(), NymNodeError> {
        todo!()
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                    trace!("ReplayProtectionBackgroundTask: Received shutdown");
                    break;
                }
            }
        }

        info!("SHUTDOWN: flushing replay detection bloomfilter to disk. this might take a while. DO NOT INTERRUPT THIS PROCESS");
        if let Err(err) = self.flush_to_disk().await {
            warn!("failed to flush replay detection bloom filter: {err}");
        }
    }
}
