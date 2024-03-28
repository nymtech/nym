// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{cache::Cache, location::LocationCacheItem};
use nym_explorer_api_requests::{Location, PrettyDetailedGatewayBond};
use nym_mixnet_contract_common::IdentityKey;
use nym_validator_client::legacy::LegacyGatewayBondWithId;
use serde::Serialize;
use std::{sync::Arc, time::SystemTime};
use tokio::sync::RwLock;

use super::location::GatewayLocationCache;

pub(crate) struct GatewayCache {
    pub(crate) gateways: Cache<IdentityKey, LegacyGatewayBondWithId>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub(crate) struct GatewaySummary {
    pub count: usize,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeGatewayCache {
    gateways: Arc<RwLock<GatewayCache>>,
    locations: Arc<RwLock<GatewayLocationCache>>,
}

impl ThreadsafeGatewayCache {
    pub(crate) fn new() -> Self {
        ThreadsafeGatewayCache {
            gateways: Arc::new(RwLock::new(GatewayCache {
                gateways: Cache::new(),
            })),
            locations: Arc::new(RwLock::new(GatewayLocationCache::new())),
        }
    }

    fn create_detailed_gateway(
        &self,
        bond: LegacyGatewayBondWithId,
        location: Option<&LocationCacheItem>,
    ) -> PrettyDetailedGatewayBond {
        PrettyDetailedGatewayBond {
            pledge_amount: bond.bond.pledge_amount,
            owner: bond.bond.owner,
            block_height: bond.bond.block_height,
            gateway: bond.bond.gateway,
            proxy: bond.bond.proxy,
            location: location.and_then(|l| l.location.clone()),
        }
    }

    pub(crate) async fn get_gateways(&self) -> Vec<LegacyGatewayBondWithId> {
        self.gateways.read().await.gateways.get_all()
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
                self.create_detailed_gateway(bond.to_owned(), location)
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
            locations: Arc::new(RwLock::new(locations)),
        }
    }

    pub(crate) async fn is_location_valid(&self, identity_key: IdentityKey) -> bool {
        self.locations
            .read()
            .await
            .get(&identity_key)
            .map_or(false, |cache_item| {
                cache_item.valid_until > SystemTime::now()
            })
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

    pub(crate) async fn update_cache(&self, gateways: Vec<LegacyGatewayBondWithId>) {
        let mut guard = self.gateways.write().await;

        for gateway in gateways {
            guard
                .gateways
                .set(gateway.gateway.identity_key.clone(), gateway)
        }
    }
}
