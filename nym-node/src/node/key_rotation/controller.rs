// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::key_rotation::manager::SphinxKeyManager;
use nym_task::ShutdownToken;
use tokio::io::AsyncWriteExt;
use tracing::{info, trace};

pub(crate) struct KeyRotationController {
    managed_keys: SphinxKeyManager,
    shutdown_token: ShutdownToken,
}

impl KeyRotationController {
    pub(crate) async fn run(&self) {
        info!("starting sphinx key rotation controller");

        while !self.shutdown_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = self.shutdown_token.cancelled() => {
                   trace!("KeyRotationController: Received shutdown");
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
