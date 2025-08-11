// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::CredentialProxyStorage;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub struct StoragePruner {
    cancellation_token: CancellationToken,
    storage: CredentialProxyStorage,
}

impl StoragePruner {
    pub fn new(cancellation_token: CancellationToken, storage: CredentialProxyStorage) -> Self {
        Self {
            cancellation_token,
            storage,
        }
    }

    pub async fn run_forever(self) {
        info!("starting the storage pruner task");
        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    break
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
