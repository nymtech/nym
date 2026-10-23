// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_families::cache::NodeFamiliesCacheData;
use crate::node_status_api::NodeStatusCache;
use crate::support::caching::cache::SharedCache;
use nym_task::cancellation::tracker::Cancelled;
use nym_task::ShutdownToken;
use std::time::Duration;
use tokio::time::{interval, sleep};
use tracing::info;

/// How often the startup wait checks its caches.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// How often the startup wait logs the caches it is still waiting for.
const LOG_INTERVAL: Duration = Duration::from_secs(30);

/// The caches nym-api builds before spawning its other tasks and serving HTTP: nothing correct
/// can be served or rewarded without them.
pub(crate) struct StartupCaches {
    pub(crate) mixnet_contract: MixnetContractCache,
    pub(crate) described_nodes: SharedCache<DescribedNodes>,
    pub(crate) node_families: SharedCache<NodeFamiliesCacheData>,
    pub(crate) node_annotations: NodeStatusCache,
}

impl StartupCaches {
    /// Returns once every cache holds a value, one loaded from a persisted file counting, logging
    /// the ones still pending; there is no time limit. Fails when shutdown is signalled first.
    pub(crate) async fn wait_until_built(
        &self,
        shutdown_token: &ShutdownToken,
    ) -> Result<(), Cancelled> {
        let mut log = interval(LOG_INTERVAL);
        loop {
            let pending = self.pending().await;
            if pending.is_empty() {
                return Ok(());
            }

            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => return Err(Cancelled),
                _ = log.tick() => info!(
                    "waiting for the startup caches to be built: {}",
                    pending.join(", ")
                ),
                _ = sleep(POLL_INTERVAL) => {}
            }
        }
    }

    /// The caches not holding a value yet.
    async fn pending(&self) -> Vec<&'static str> {
        let mut pending = Vec::new();
        if self.mixnet_contract.inner().get().await.is_err() {
            pending.push("mixnet contract");
        }
        if self.described_nodes.get().await.is_err() {
            pending.push("described nodes");
        }
        if self.node_families.get().await.is_err() {
            pending.push("node families");
        }
        if self.node_annotations.cache().await.is_err() {
            pending.push("node annotations");
        }
        pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mixnet_contract_cache::cache::data::test_helpers::populated_cache;
    use crate::mixnet_contract_cache::cache::data::MixnetContractCacheData;
    use crate::node_status_api::cache::data::NodeStatusCacheData;
    use std::collections::HashMap;

    /// The four caches' shared handles, to fill them while the wait runs.
    #[derive(Default)]
    struct Handles {
        mixnet_contract: SharedCache<MixnetContractCacheData>,
        described_nodes: SharedCache<DescribedNodes>,
        node_families: SharedCache<NodeFamiliesCacheData>,
        node_annotations: SharedCache<NodeStatusCacheData>,
    }

    impl Handles {
        fn startup_caches(&self) -> StartupCaches {
            StartupCaches {
                mixnet_contract: self.mixnet_contract.clone().into(),
                described_nodes: self.described_nodes.clone(),
                node_families: self.node_families.clone(),
                node_annotations: self.node_annotations.clone().into(),
            }
        }

        async fn build_all_but_the_annotations(&self) {
            let described_nodes = DescribedNodes {
                nodes: HashMap::new(),
                addresses_cache: HashMap::new(),
            };
            let built = [
                self.mixnet_contract
                    .try_overwrite_old_value(populated_cache(), "test")
                    .await
                    .is_ok(),
                self.described_nodes
                    .try_overwrite_old_value(described_nodes, "test")
                    .await
                    .is_ok(),
                self.node_families
                    .try_overwrite_old_value(NodeFamiliesCacheData::default(), "test")
                    .await
                    .is_ok(),
            ];
            assert_eq!(built, [true; 3]);
        }

        async fn build_the_annotations(&self) {
            let built = self
                .node_annotations
                .try_overwrite_old_value(HashMap::new(), "test")
                .await;
            assert!(built.is_ok());
        }
    }

    #[tokio::test(start_paused = true)]
    async fn the_wait_returns_once_every_cache_holds_a_value_and_not_before() {
        let handles = Handles::default();
        let caches = handles.startup_caches();
        let wait =
            tokio::spawn(async move { caches.wait_until_built(&ShutdownToken::new()).await });

        handles.build_all_but_the_annotations().await;
        sleep(Duration::from_secs(60)).await;
        assert!(!wait.is_finished());

        handles.build_the_annotations().await;
        sleep(2 * POLL_INTERVAL).await;
        assert!(wait.is_finished());
        assert!(wait.await.unwrap().is_ok());
    }

    /// As every cache is after loading a persisted file within its age limit.
    #[tokio::test(start_paused = true)]
    async fn caches_already_holding_values_need_no_refresh() {
        let handles = Handles::default();
        handles.build_all_but_the_annotations().await;
        handles.build_the_annotations().await;

        let start = tokio::time::Instant::now();
        let waited = handles
            .startup_caches()
            .wait_until_built(&ShutdownToken::new())
            .await;
        assert!(waited.is_ok());
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn a_shutdown_ends_the_wait() {
        let shutdown = ShutdownToken::new();
        let caches = Handles::default().startup_caches();
        let wait = tokio::spawn({
            let shutdown = shutdown.clone();
            async move { caches.wait_until_built(&shutdown).await }
        });

        sleep(Duration::from_secs(60)).await;
        shutdown.cancel();
        assert!(wait.await.unwrap().is_err());
    }
}
