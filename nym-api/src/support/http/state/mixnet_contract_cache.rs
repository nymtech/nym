// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::support::caching::refresher::RefreshRequester;
use crate::support::http::state::helpers::Refreshing;

#[derive(Clone)]
pub(crate) struct MixnetContractCacheState {
    pub(crate) inner_cache: MixnetContractCache,
    pub(crate) refresh_handle: Refreshing,
}

impl MixnetContractCacheState {
    pub(crate) fn new(inner_cache: MixnetContractCache, refresh_handle: RefreshRequester) -> Self {
        MixnetContractCacheState {
            inner_cache,
            refresh_handle: Refreshing::new(refresh_handle),
        }
    }
}
