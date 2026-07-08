// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NyxdFetcherError;

use nym_ecash_time::OffsetDateTime;
use nym_validator_client::EcashApiClient;
use nym_validator_client::coconut::{EcashApiError, all_ecash_api_clients};
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;

pub use public_data::NyxdGlobalDataFetcher;

#[cfg(not(target_arch = "wasm32"))]
pub use credentials::NyxdCredentialFetcher;
#[cfg(all(not(target_arch = "wasm32"), feature = "recovery"))]
pub use credentials::recovery::NyxdRecoveryFetcher;

// credential issuance/recovery is backed by a sqlite pending-requests store, so it's native-only.
// wasm only uses the storage-free `NyxdGlobalDataFetcher`.
#[cfg(not(target_arch = "wasm32"))]
mod credentials;
mod error;
mod public_data;
#[cfg(not(target_arch = "wasm32"))]
mod storage;

// Per-epoch cache for the ecash api clients, lazily populated. Keeping one entry per epoch lets
// us serve several epochs at once instead of thrashing a single entry when requests interleave
// across an epoch boundary. Each epoch gets its own async entry: concurrent requests for the same
// epoch serialise on it so only one performs the fetch and the rest observe the fresh result,
// while different epochs never block each other. The outer std mutex only guards the map
// structure and is never held across an await.
struct EcashApiClientsCache {
    entries: Mutex<HashMap<EpochId, CacheEntry>>,
}

type CacheEntry = Arc<AsyncMutex<Option<EcashApiClientsCacheInner>>>;

impl EcashApiClientsCache {
    fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
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
        // if the mutex was poisoned by a panic mid-update, just drop the whole cache and start
        // fresh - the entries are only a fetch away from being rebuilt.
        let entry = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| {
                let mut guard = poisoned.into_inner();
                guard.clear();
                guard
            })
            .entry(epoch_id)
            .or_default()
            .clone();

        // hold the per-epoch entry across the fetch so a concurrent request for this epoch waits
        // here rather than issuing a redundant fetch.
        let mut guard = entry.lock().await;
        if let Some(cache) = guard.as_ref() {
            if !cache.is_stale() {
                return Ok(cache.clients.clone());
            }
        }

        // empty or stale - refresh and cache it.
        let cache = EcashApiClientsCacheInner::from_dkg_client(query_client, epoch_id).await?;
        let clients = cache.clients.clone();
        *guard = Some(cache);
        Ok(clients)
    }
}

struct EcashApiClientsCacheInner {
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
            clients,
            last_updated_at: OffsetDateTime::now_utc(),
        })
    }
}
