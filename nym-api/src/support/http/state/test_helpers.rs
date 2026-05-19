// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Shared test-only helpers for assembling an [`AppState`] without going
//! through the full nym-api startup path.

use crate::ecash::state::EcashState;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::network::models::NetworkDetails;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_families::cache::NodeFamiliesCacheData;
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::NodeStatusCache;
use crate::status::ApiStatusState;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::http::state::chain_status::ChainStatusCache;
use crate::support::http::state::contract_details::ContractDetailsCache;
use crate::support::http::state::force_refresh::ForcedRefresh;
use crate::support::http::state::mixnet_contract_cache::MixnetContractCacheState;
use crate::support::http::state::node_annotations_cache::NodeAnnotationsCache;
use crate::support::http::state::AppState;
use crate::support::nyxd::Client;
use crate::support::storage::NymApiStorage;
use crate::unstable_routes::v1::account::cache::AddressInfoCache;
use nym_config::defaults::NymNetworkDetails;
use std::sync::Arc;
use std::time::Duration;

/// Construct a default test [`AppState`]. All caches start empty
/// (in-memory `SharedCache::new()` for the disk-backed ones); seed
/// `node_families_cache` upstream of this call if the test needs data.
pub(crate) fn build_app_state(
    storage: NymApiStorage,
    ecash_state: EcashState,
    nyxd_client: Client,
    node_families_cache: SharedCache<NodeFamiliesCacheData>,
) -> AppState {
    let mixnet_contract_cache: MixnetContractCache = SharedCache::new().into();
    let mixnet_contract_cache =
        MixnetContractCacheState::new(mixnet_contract_cache, RefreshRequester::default());

    let node_status_cache: NodeStatusCache = SharedCache::new().into();
    let node_annotations_cache =
        NodeAnnotationsCache::new(node_status_cache, RefreshRequester::default());

    AppState {
        nyxd_client,
        chain_status_cache: ChainStatusCache::new(Duration::from_secs(42)),
        ecash_signers_cache: Default::default(),
        address_info_cache: AddressInfoCache::new(Duration::from_secs(42), 1000),
        forced_refresh: ForcedRefresh::new(true),
        mixnet_contract_cache,
        node_families_cache,
        node_annotations_cache,
        storage,
        described_nodes_cache: SharedCache::<DescribedNodes>::new(),
        network_details: NetworkDetails::new(
            "localhost".to_string(),
            NymNetworkDetails::new_empty(),
        ),
        node_info_cache: unstable::NodeInfoCache::default(),
        contract_info_cache: ContractDetailsCache::new(Duration::from_secs(42)),
        api_status: ApiStatusState::new(None),
        ecash_state: Arc::new(ecash_state),
    }
}
