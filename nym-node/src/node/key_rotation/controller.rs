// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::key_rotation::manager::SphinxKeyManager;
use crate::node::nym_apis_client::NymApisClient;
use nym_task::ShutdownToken;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, trace};

pub(crate) struct KeyRotationController {
    // regular polling rate to catch any changes in the system config. they shouldn't happen too often
    // so the requests can be sent quite infrequently
    regular_polling_interval: Duration,

    client: NymApisClient,
    managed_keys: SphinxKeyManager,
    shutdown_token: ShutdownToken,
}

enum KeyRotationActionState {
    // perform key-rotation and pre-announce new key to the nym-api(s)
    PreAnnounce,

    // remove the old key and purge associated data like the replay detection bloomfilter
    PurgeOld,
}

impl KeyRotationController {
    pub(crate) fn new(client: NymApisClient, shutdown_token: ShutdownToken) -> Self {
        todo!()
    }

    async fn regular_poll(&self) {
        todo!()
    }

    pub(crate) async fn run(&self) {
        info!("starting sphinx key rotation controller");

        let mut polling_interval = interval(self.regular_polling_interval);
        polling_interval.reset();

        while !self.shutdown_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                   trace!("KeyRotationController: Received shutdown");
                }
                _ = polling_interval.tick() => {
                    self.regular_poll().await;
                }
                // TODO:
            }
        }

        trace!("KeyRotationController: exiting")
    }

    pub(crate) fn start(self) {
        tokio::spawn(async move { self.run().await });
    }
}
