// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use nym_sdk::{NymApiTopologyProvider, NymApiTopologyProviderConfig, UserAgent};
use nym_topology::{gateway, NymTopology, TopologyProvider};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;
use url::Url;

#[derive(Clone)]
pub struct GatewayTopologyProvider {
    inner: Arc<Mutex<GatewayTopologyProviderInner>>,
}

impl GatewayTopologyProvider {
    pub fn new(
        gateway_node: gateway::LegacyNode,
        user_agent: UserAgent,
        nym_api_url: Vec<Url>,
    ) -> GatewayTopologyProvider {
        GatewayTopologyProvider {
            inner: Arc::new(Mutex::new(GatewayTopologyProviderInner {
                inner: NymApiTopologyProvider::new(
                    NymApiTopologyProviderConfig {
                        min_mixnode_performance: 50,
                        min_gateway_performance: 0,
                    },
                    nym_api_url,
                    env!("CARGO_PKG_VERSION").to_string(),
                    Some(user_agent),
                ),
                gateway_node,
            })),
        }
    }
}

struct GatewayTopologyProviderInner {
    inner: NymApiTopologyProvider,
    gateway_node: gateway::LegacyNode,
}

#[async_trait]
impl TopologyProvider for GatewayTopologyProvider {
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        let mut guard = self.inner.lock().await;
        match guard.inner.get_new_topology().await {
            None => None,
            Some(mut base) => {
                if !base.gateway_exists(&guard.gateway_node.identity_key) {
                    debug!(
                        "{} didn't exist in topology. inserting it.",
                        guard.gateway_node.identity_key
                    );
                    base.insert_gateway(guard.gateway_node.clone());
                }
                Some(base)
            }
        }
    }
}
