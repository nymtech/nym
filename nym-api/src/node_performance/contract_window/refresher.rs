// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_performance::contract_window::PerformanceContractWindow;
use anyhow::bail;
use nym_task::ShutdownToken;
use nym_validator_client::QueryHttpRpcNyxdClient;
use std::time::Duration;
use tokio::time::interval;
use tracing::{trace, warn};

/// Keeps the window current, loading it on its first poll unless startup already did.
pub(crate) struct PerformanceContractWindowRefresher {
    window: PerformanceContractWindow,
    query_client: QueryHttpRpcNyxdClient,
    mixnet_contract_cache: MixnetContractCache,
    polling_interval: Duration,
}

impl PerformanceContractWindowRefresher {
    pub(crate) fn new(
        window: PerformanceContractWindow,
        query_client: QueryHttpRpcNyxdClient,
        mixnet_contract_cache: MixnetContractCache,
        polling_interval: Duration,
    ) -> Self {
        PerformanceContractWindowRefresher {
            window,
            query_client,
            mixnet_contract_cache,
            polling_interval,
        }
    }

    /// Loads the window and requires it to hold data; contract mode awaits this before anything
    /// can reward. An empty contract would pay every node zero and leave the rewarded set empty,
    /// and checking later, mid epoch transition, would deadlock: the contract accepts no
    /// submission while the epoch is in transition.
    pub(crate) async fn load_initial_values(&self) -> anyhow::Result<()> {
        self.mixnet_contract_cache
            .naive_wait_for_initial_values()
            .await;
        self.poll().await?;

        if !self
            .window
            .read()
            .await
            .is_some_and(|data| data.has_submission())
        {
            bail!("the performance contract has never received a submission, so nothing can be rewarded from it")
        }
        Ok(())
    }

    pub(crate) fn start(self, shutdown_token: ShutdownToken) {
        tokio::spawn(async move { self.run(shutdown_token).await });
    }

    async fn run(self, shutdown_token: ShutdownToken) {
        // X comes from the mixnet contract cache
        tokio::select! {
            _ = self.mixnet_contract_cache.naive_wait_for_initial_values() => {}
            _ = shutdown_token.cancelled() => {
                trace!("PerformanceContractWindowRefresher: Received shutdown");
                return;
            }
        }

        let mut polling_interval = interval(self.polling_interval);
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("PerformanceContractWindowRefresher: Received shutdown");
                    break;
                }
                _ = polling_interval.tick() => {
                    if let Err(err) = self.poll().await {
                        warn!("failed to refresh the performance contract window: {err:#}");
                    }
                }
            }
        }
    }

    async fn poll(&self) -> anyhow::Result<()> {
        let current_epoch = self
            .mixnet_contract_cache
            .current_interval()
            .await?
            .current_epoch_absolute_id();
        self.window.poll(&self.query_client, current_epoch).await
    }
}
