// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cache::Cache;
use mixnet_contract_common::{GatewayBond, IdentityKey};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) struct GatewayCache {
    pub(crate) gateways: Cache<IdentityKey, GatewayBond>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct GatewaySummary {
    pub count: usize,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeGatewayCache {
    inner: Arc<RwLock<GatewayCache>>,
}

impl ThreadsafeGatewayCache {
    pub(crate) fn new() -> Self {
        ThreadsafeGatewayCache {
            inner: Arc::new(RwLock::new(GatewayCache {
                gateways: Cache::new(),
            })),
        }
    }

    pub(crate) async fn get_gateways(&self) -> Vec<GatewayBond> {
        self.inner.read().await.gateways.get_all()
    }

    pub(crate) async fn get_gateway_summary(&self) -> GatewaySummary {
        GatewaySummary {
            count: self.inner.read().await.gateways.len(),
        }
    }

    pub(crate) async fn update_cache(&self, gateways: Vec<GatewayBond>) {
        let mut guard = self.inner.write().await;

        for gateway in gateways {
            guard
                .gateways
                .set(gateway.gateway.identity_key.clone(), gateway)
        }
    }
}
