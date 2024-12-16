// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use nym_gateway::node::{NymApiTopologyProvider, NymApiTopologyProviderConfig, UserAgent};
use nym_topology::node::RoutingNode;
use nym_topology::{NymTopology, Role, TopologyProvider};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use tracing::debug;
use url::Url;

// I wouldn't be surprised if this became the start of the node topology cache

#[derive(Clone)]
pub struct NymNodeTopologyProvider {
    inner: Arc<Mutex<NymNodeTopologyProviderInner>>,
}

impl NymNodeTopologyProvider {
    pub fn new(
        gateway_node: RoutingNode,
        cache_ttl: Duration,
        user_agent: UserAgent,
        nym_api_url: Vec<Url>,
    ) -> NymNodeTopologyProvider {
        NymNodeTopologyProvider {
            inner: Arc::new(Mutex::new(NymNodeTopologyProviderInner {
                inner: NymApiTopologyProvider::new(
                    NymApiTopologyProviderConfig {
                        min_mixnode_performance: 50,
                        min_gateway_performance: 0,
                        use_extended_topology: false,
                        ignore_egress_epoch_role: true,
                    },
                    nym_api_url,
                    Some(user_agent),
                ),
                cache_ttl,
                cached_at: OffsetDateTime::UNIX_EPOCH,
                cached: None,
                gateway_node,
            })),
        }
    }
}

struct NymNodeTopologyProviderInner {
    inner: NymApiTopologyProvider,
    cache_ttl: Duration,
    cached_at: OffsetDateTime,
    cached: Option<NymTopology>,
    gateway_node: RoutingNode,
}

impl NymNodeTopologyProviderInner {
    fn cached_topology(&self) -> Option<NymTopology> {
        if let Some(cached_topology) = &self.cached {
            if self.cached_at + self.cache_ttl > OffsetDateTime::now_utc() {
                return Some(cached_topology.clone());
            }
        }

        None
    }

    async fn update_cache(&mut self) -> Option<NymTopology> {
        let updated_cache = match self.inner.get_new_topology().await {
            None => None,
            Some(mut base) => {
                if !base.has_node_details(self.gateway_node.node_id) {
                    debug!(
                        "{} didn't exist in topology. inserting it.",
                        self.gateway_node.identity_key
                    );
                    base.insert_node_details(self.gateway_node.clone());
                }
                base.force_set_active(self.gateway_node.node_id, Role::EntryGateway);
                Some(base)
            }
        };

        self.cached_at = OffsetDateTime::now_utc();
        self.cached = updated_cache.clone();

        updated_cache
    }
}

#[async_trait]
impl TopologyProvider for NymNodeTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        let mut guard = self.inner.lock().await;
        // check the cache
        if let Some(cached) = guard.cached_topology() {
            return Some(cached);
        }
        guard.update_cache().await
    }
}
