// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::VpnApiStorage;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub struct StoragePruner {
    cancellation_token: CancellationToken,
    storage: VpnApiStorage,
}

impl StoragePruner {
    pub fn new(cancellation_token: CancellationToken, storage: VpnApiStorage) -> Self {
        Self {
            cancellation_token,
            storage,
        }
    }

    pub async fn run_forever(self) {
        while !self.cancellation_token.is_cancelled() {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    // The token was cancelled, task can shut down
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(60 * 60)) => {
                    match self.storage.prune_old_blinded_shares().await {
                        Ok(_res) => info!("🧹 Pruning old blinded shares complete"),
                        Err(err) => error!("Failed to prune old blinded shares: {err}"),
                    }
                }
            }
        }
    }
}
