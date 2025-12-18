// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet_contract_cache::cache::data::MixnetContractCacheData;
use crate::mixnet_contract_cache::cache::refresher::MixnetContractDataProvider;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::support::caching::refresher::CacheRefresher;
use crate::support::{config, nyxd};
use nym_validator_client::nyxd::error::NyxdError;
use std::path::PathBuf;

pub(crate) mod cache;
pub(crate) mod handlers;

pub(crate) fn build_refresher(
    config: &config::MixnetContractCache,
    nym_contract_cache_state: &MixnetContractCache,
    nyxd_client: nyxd::Client,
    on_disk_file: PathBuf,
) -> CacheRefresher<MixnetContractCacheData, NyxdError> {
    CacheRefresher::new_with_initial_value(
        Box::new(MixnetContractDataProvider::new(nyxd_client)),
        config.debug.caching_interval,
        nym_contract_cache_state.inner(),
    )
    .named("mixnet-contract-cache-refresher")
    .with_persistent_cache(on_disk_file)
}
