// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::NodeStatusCache;
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::SharedCache;
use crate::support::storage::NymApiStorage;
use axum::extract::FromRef;

#[derive(Clone)]
pub struct AppState {
    pub contract_cache: NymContractCache,
    pub status_cache: NodeStatusCache,
    pub circulating_supply_cache: CirculatingSupplyCache,
    pub storage: NymApiStorage,
    pub describes_nodes_state: SharedCache<DescribedNodes>,
}

impl FromRef<AppState> for NymContractCache {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.contract_cache.clone()
    }
}

impl FromRef<AppState> for NodeStatusCache {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.status_cache.clone()
    }
}

impl FromRef<AppState> for CirculatingSupplyCache {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.circulating_supply_cache.clone()
    }
}

impl FromRef<AppState> for NymApiStorage {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.storage.clone()
    }
}

impl FromRef<AppState> for SharedCache<DescribedNodes> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.describes_nodes_state.clone()
    }
}
