// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwapOption;
use nym_ecash_time::OffsetDateTime;
use nym_validator_client::coconut::{all_ecash_api_clients, EcashApiError};
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::EcashApiClient;

#[cfg(all(not(target_arch = "wasm32"), feature = "recovery"))]
pub use credentials::recovery::NyxdRecoveryFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub use credentials::NyxdCredentialFetcher;
pub use public_data::NyxdGlobalDataFetcher;

use crate::error::NyxdFetcherError;

// credential issuance/recovery is backed by a sqlite pending-requests store, so it's native-only.
// wasm only uses the storage-free `NyxdGlobalDataFetcher`.
#[cfg(not(target_arch = "wasm32"))]
mod credentials;
mod error;
mod public_data;
#[cfg(not(target_arch = "wasm32"))]
mod storage;

// Lock-free cache for the ecash api clients, lazily instantiated. `ArcSwapOption` lets us
// refresh it through `&self` (so the trait methods can stay `&self`) without a mutex.
struct EcashApiClientsCache {
    inner: ArcSwapOption<EcashApiClientsCacheInner>,
}

impl EcashApiClientsCache {
    fn new() -> Self {
        Self {
            inner: ArcSwapOption::empty(),
        }
    }
    async fn get<C>(
        &self,
        query_client: &C,
        epoch_id: EpochId,
    ) -> Result<Vec<EcashApiClient>, NyxdFetcherError>
    where
        C: DkgQueryClient,
    {
        // fast path: atomic load, return if cached for this epoch
        if let Some(cache) = self.inner.load_full() {
            if cache.epoch_id == epoch_id && !cache.is_stale() {
                return Ok(cache.clients.clone());
            }
        }

        // empty or stale - refresh and atomically swap in the new cache, then return it.
        // a concurrent miss may fetch redundantly and the later store wins; harmless since
        // the fetch is idempotent.
        let cache =
            Arc::new(EcashApiClientsCacheInner::from_dkg_client(query_client, epoch_id).await?);
        let clients = cache.clients.clone();
        self.inner.store(Some(cache));
        Ok(clients)
    }
}

struct EcashApiClientsCacheInner {
    epoch_id: EpochId,
    clients: Vec<EcashApiClient>,
    last_updated_at: OffsetDateTime,
}

impl EcashApiClientsCacheInner {
    const VALIDITY_DURATION: Duration = Duration::from_secs(30 * 60); // 30 minutes

    fn is_stale(&self) -> bool {
        self.last_updated_at + Self::VALIDITY_DURATION < OffsetDateTime::now_utc()
    }

    async fn from_dkg_client<C>(query_client: &C, epoch_id: EpochId) -> Result<Self, EcashApiError>
    where
        C: DkgQueryClient,
    {
        let clients = all_ecash_api_clients(query_client, epoch_id).await?;
        Ok(EcashApiClientsCacheInner {
            epoch_id,
            clients,
            last_updated_at: OffsetDateTime::now_utc(),
        })
    }
}
