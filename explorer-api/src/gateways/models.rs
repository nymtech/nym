// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{cache::Cache, location::LocationCacheItem};
use nym_contracts_common::IdentityKey;
use nym_explorer_api_requests::{Location, PrettyDetailedGatewayBond};
use nym_mixnet_contract_common::GatewayBond;
use nym_validator_client::models::GatewayBondAnnotated;
use serde::Serialize;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::RwLock;

use super::location::GatewayLocationCache;

pub(crate) struct GatewayCache {
    pub(crate) gateways: Cache<IdentityKey, GatewayBondAnnotated>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct GatewaySummary {
    pub count: usize,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeGatewayCache {
    gateways: Arc<RwLock<GatewayCache>>,
    legacy_gateway_bonds: Arc<RwLock<GatewayCache>>,
    locations: Arc<RwLock<GatewayLocationCache>>,
}

impl ThreadsafeGatewayCache {
    pub(crate) fn new() -> Self {
        ThreadsafeGatewayCache {
            gateways: Arc::new(RwLock::new(GatewayCache {
                gateways: Cache::new(),
            })),
            legacy_gateway_bonds: Arc::new(RwLock::new(GatewayCache {
                gateways: Cache::new(),
            })),
            locations: Arc::new(RwLock::new(GatewayLocationCache::new())),
        }
    }

    fn create_detailed_gateway(
        &self,
        bond: GatewayBond,
        location: Option<&LocationCacheItem>,
    ) -> PrettyDetailedGatewayBond {
        PrettyDetailedGatewayBond {
            pledge_amount: bond.pledge_amount,
            owner: bond.owner,
            block_height: bond.block_height,
            gateway: bond.gateway,
            proxy: bond.proxy,
            location: location.and_then(|l| l.location.clone()),
        }
    }

    pub(crate) async fn get_gateways(&self) -> Vec<GatewayBond> {
        self.gateways
            .read()
            .await
            .gateways
            .get_all()
            .iter()
            .map(|g| g.gateway_bond.bond.clone())
            .collect()
    }

    pub(crate) async fn get_detailed_gateways(&self) -> Vec<PrettyDetailedGatewayBond> {
        let gateways_guard = self.gateways.read().await;
        let location_guard = self.locations.read().await;

        gateways_guard
            .gateways
            .get_all()
            .iter()
            .map(|bond| {
                let location = location_guard.get(bond.identity());
                self.create_detailed_gateway(bond.gateway_bond.bond.to_owned(), location)
            })
            .collect()
    }

    pub(crate) async fn get_legacy_detailed_gateways(&self) -> Vec<PrettyDetailedGatewayBond> {
        let legacy_gateways = self.legacy_gateway_bonds.read().await;
        let location_guard = self.locations.read().await;

        legacy_gateways
            .gateways
            .get_all()
            .iter()
            .map(|bond| {
                let location = location_guard.get(bond.identity());
                self.create_detailed_gateway(bond.gateway_bond.bond.to_owned(), location)
            })
            .collect()
    }

    pub(crate) async fn get_gateway_summary(&self) -> GatewaySummary {
        GatewaySummary {
            count: self.gateways.read().await.gateways.len(),
        }
    }

    pub(crate) fn new_with_location_cache(locations: GatewayLocationCache) -> Self {
        ThreadsafeGatewayCache {
            gateways: Arc::new(RwLock::new(GatewayCache {
                gateways: Cache::new(),
            })),
            legacy_gateway_bonds: Arc::new(RwLock::new(GatewayCache {
                gateways: Cache::new(),
            })),
            locations: Arc::new(RwLock::new(locations)),
        }
    }

    pub(crate) async fn is_location_valid(&self, identity_key: IdentityKey) -> bool {
        self.locations
            .read()
            .await
            .get(&identity_key)
            .is_some_and(|cache_item| cache_item.valid_until > SystemTime::now())
    }

    pub(crate) async fn get_locations(&self) -> GatewayLocationCache {
        self.locations.read().await.clone()
    }

    pub(crate) async fn set_location(&self, identy_key: IdentityKey, location: Option<Location>) {
        // cache the location for this mix node so that it can be used when the mix node list is refreshed
        self.locations
            .write()
            .await
            .insert(identy_key, LocationCacheItem::new_from_location(location));
    }

    pub(crate) async fn update_cache(
        &self,
        gateways: Vec<GatewayBondAnnotated>,
        legacy_gateway_bonds: Vec<GatewayBond>,
    ) {
        let mut guard = self.gateways.write().await;
        let mut guard_legacy_gateways = self.legacy_gateway_bonds.write().await;

        for gateway in gateways {
            guard
                .gateways
                .set(gateway.gateway_bond.gateway.identity_key.clone(), gateway)
        }

        for legacy_gateway in legacy_gateway_bonds {
            if let Some(g) = guard.gateways.get(&legacy_gateway.gateway.identity_key) {
                guard_legacy_gateways
                    .gateways
                    .set(legacy_gateway.gateway.identity_key, g.clone());
            }
        }
    }
}
