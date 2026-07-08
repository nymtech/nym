// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_bandwidth_controller::{CredentialFetcherError, CredentialPublicDataFetcher};
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_ecash_time::Date;
use nym_validator_client::EcashApiClient;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use tracing::debug;

use crate::EcashApiClientsCache;
use crate::error::NyxdFetcherError;

/// In-repo [`CredentialPublicDataFetcher`] that retrieves the global ecash signing materials from the
/// nym-apis using a nyxd query client.
pub struct NyxdGlobalDataFetcher<C> {
    client: Arc<C>,

    ecash_api_clients: Arc<EcashApiClientsCache>,
}

impl<C: DkgQueryClient> NyxdGlobalDataFetcher<C> {
    pub fn new(client: Arc<C>) -> Self {
        NyxdGlobalDataFetcher {
            client,
            ecash_api_clients: Arc::new(EcashApiClientsCache::new()),
        }
    }

    // only the native credential fetcher shares its ecash-clients cache this way
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn new_with_ecash_clients(
        client: Arc<C>,
        ecash_api_clients: Arc<EcashApiClientsCache>,
    ) -> Self {
        NyxdGlobalDataFetcher {
            client,
            ecash_api_clients,
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C: DkgQueryClient> CredentialPublicDataFetcher for NyxdGlobalDataFetcher<C> {
    async fn fetch_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<EpochVerificationKey, CredentialFetcherError> {
        let ecash_apis = self.ecash_api_clients.get(&*self.client, epoch_id).await?;
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
    ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
        let ecash_apis = self.ecash_api_clients.get(&*self.client, epoch_id).await?;
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
    ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
        let ecash_apis = self.ecash_api_clients.get(&*self.client, epoch_id).await?;
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
) -> Result<T, NyxdFetcherError>
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
                debug!("failed to obtain valid response from API {disp}: {err}")
            }
        }
    }
    Err(NyxdFetcherError::ExhaustedApiQueries { typ: typ.into() })
}
