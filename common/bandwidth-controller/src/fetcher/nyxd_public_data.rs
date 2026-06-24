// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use arc_swap::ArcSwapOption;
use async_trait::async_trait;
use log::warn;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_ecash_time::Date;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::EcashApiClient;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use thiserror::Error;

use crate::fetcher::EcashApiClientsCache;
use crate::traits::{CredentialPublicDataFetcher, FetcherError};

/// In-repo [`CredentialPublicDataFetcher`] that retrieves the global ecash signing materials from the
/// nym-apis using a nyxd query client.
pub struct NyxdGlobalDataFetcher<C: DkgQueryClient + Send + Sync> {
    client: C,

    // Lock-free cache for the ecash api clients, lazily instantiated. `ArcSwapOption` lets us
    // refresh it through `&self` (so the trait methods can stay `&self`) without a mutex.
    ecash_api_clients: ArcSwapOption<EcashApiClientsCache>,
}

impl<C: DkgQueryClient + Send + Sync> NyxdGlobalDataFetcher<C> {
    pub fn new(client: C) -> Self {
        NyxdGlobalDataFetcher {
            client,
            ecash_api_clients: ArcSwapOption::empty(),
        }
    }

    async fn ecash_api_clients(&self, epoch_id: EpochId) -> Result<Vec<EcashApiClient>, Error> {
        // fast path: atomic load, return if cached for this epoch
        if let Some(cache) = self.ecash_api_clients.load_full() {
            if cache.epoch_id == epoch_id && !cache.is_stale() {
                return Ok(cache.clients.clone());
            }
        }

        // empty or stale - refresh and atomically swap in the new cache, then return it.
        // a concurrent miss may fetch redundantly and the later store wins; harmless since
        // the fetch is idempotent.
        let cache = Arc::new(EcashApiClientsCache::from_dkg_client(&self.client, epoch_id).await?);
        let clients = cache.clients.clone();
        self.ecash_api_clients.store(Some(cache));
        Ok(clients)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C: DkgQueryClient + Send + Sync> CredentialPublicDataFetcher for NyxdGlobalDataFetcher<C> {
    async fn fetch_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<EpochVerificationKey, FetcherError> {
        let ecash_apis = self.ecash_api_clients(epoch_id).await?;
        let master_vk = query_random_apis_until_success(
            ecash_apis,
            |api| async move { api.api_client.master_verification_key(Some(epoch_id)).await },
            format!("aggregated verification key for epoch {epoch_id}"),
        )
        .await?
        .key;

        Ok(EpochVerificationKey {
            epoch_id,
            key: master_vk,
        })
    }

    async fn fetch_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<AggregatedCoinIndicesSignatures, FetcherError> {
        let ecash_apis = self.ecash_api_clients(epoch_id).await?;
        let index_sigs = query_random_apis_until_success(
            ecash_apis,
            |api| async move {
                api.api_client
                    .global_coin_indices_signatures(Some(epoch_id))
                    .await
            },
            format!("aggregated coin index signatures for epoch {epoch_id}"),
        )
        .await?
        .signatures;

        Ok(AggregatedCoinIndicesSignatures {
            epoch_id,
            signatures: index_sigs,
        })
    }

    async fn fetch_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<AggregatedExpirationDateSignatures, FetcherError> {
        let ecash_apis = self.ecash_api_clients(epoch_id).await?;
        let expiration_sigs = query_random_apis_until_success(
            ecash_apis,
            |api| async move {
                api.api_client
                    .global_expiration_date_signatures(Some(expiration_date), Some(epoch_id))
                    .await
            },
            format!("aggregated expiration date signatures for date {expiration_date}"),
        )
        .await?
        .signatures;

        Ok(AggregatedExpirationDateSignatures {
            epoch_id,
            expiration_date,
            signatures: expiration_sigs,
        })
    }
}

async fn query_random_apis_until_success<F, T, U, E>(
    mut apis: Vec<EcashApiClient>,
    f: F,
    typ: impl Into<String>,
) -> Result<T, Error>
where
    F: Fn(EcashApiClient) -> U,
    U: Future<Output = Result<T, E>>,
    E: Display,
{
    // try apis in pseudorandom way to remove any bias towards the first registered dealer
    apis.shuffle(&mut thread_rng());

    for api in apis {
        let disp = api.to_string();
        match f(api).await {
            Ok(res) => return Ok(res),
            Err(err) => {
                warn!("failed to obtain valid response from API {disp}: {err}")
            }
        }
    }
    Err(Error::ExhaustedApiQueries { typ: typ.into() })
}

#[derive(Error, Debug)]
enum Error {
    #[error("Nyxd error: {0}")]
    Nyxd(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error("ecash api query failure: {0}")]
    EcashApiError(#[from] EcashApiError),

    #[error("did not receive a valid response for aggregated data ({typ}) from ANY nym-api")]
    ExhaustedApiQueries { typ: String },
}
