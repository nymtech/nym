// Copyright 2025 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::nym_api_helpers::{
    ensure_sane_expiration_date, query_all_threshold_apis, CachedEpoch, CachedImmutableEpochItem,
};
use crate::quorum_checker::QuorumState;
use crate::shared_state::nyxd_client::ChainClient;
use crate::shared_state::required_deposit_cache::RequiredDepositCache;
use crate::storage::traits::GlobalEcashDataCache;
use nym_cache::CachedImmutableItems;
use nym_compact_ecash::scheme::coin_indices_signatures::aggregate_annotated_indices_signatures;
use nym_compact_ecash::scheme::expiration_date_signatures::aggregate_annotated_expiration_signatures;
use nym_credentials::ecash::utils::EcashTime;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::EcashApiClient;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::info;

pub use nym_compact_ecash::scheme::coin_indices_signatures::CoinIndexSignatureShare;
pub use nym_compact_ecash::scheme::expiration_date_signatures::ExpirationDateSignatureShare;
pub use nym_compact_ecash::VerificationKeyAuth;
pub use nym_credentials::{IssuanceTicketBook, IssuedTicketBook};
pub use nym_credentials_interface::{TicketType, TicketTypeRepr};

pub struct EcashState {
    pub required_deposit_cache: RequiredDepositCache,

    pub quorum_state: QuorumState,

    pub cached_epoch: RwLock<CachedEpoch>,

    pub master_verification_key: CachedImmutableEpochItem<VerificationKeyAuth>,

    pub threshold_values: CachedImmutableEpochItem<u64>,

    pub epoch_clients: CachedImmutableEpochItem<Vec<EcashApiClient>>,

    pub coin_index_signatures: CachedImmutableEpochItem<AggregatedCoinIndicesSignatures>,

    pub expiration_date_signatures:
        CachedImmutableItems<(EpochId, Date), AggregatedExpirationDateSignatures>,
}

impl EcashState {
    pub fn new(
        required_deposit_cache: RequiredDepositCache,
        quorum_state: QuorumState,
    ) -> EcashState {
        EcashState {
            required_deposit_cache,
            quorum_state,
            cached_epoch: Default::default(),
            master_verification_key: Default::default(),
            threshold_values: Default::default(),
            epoch_clients: Default::default(),
            coin_index_signatures: Default::default(),
            expiration_date_signatures: Default::default(),
        }
    }

    pub async fn ensure_credentials_issuable(
        &self,
        client: &ChainClient,
    ) -> Result<(), CredentialProxyError> {
        let epoch = self.current_epoch(client).await?;

        if epoch.state.is_final() {
            Ok(())
        } else if let Some(final_timestamp) = epoch.final_timestamp_secs() {
            // SAFETY: the timestamp values in our DKG contract should be valid timestamps,
            // otherwise it means the chain is seriously misbehaving
            #[allow(clippy::unwrap_used)]
            let finish_dt = OffsetDateTime::from_unix_timestamp(final_timestamp as i64).unwrap();

            Err(CredentialProxyError::CredentialsNotYetIssuable {
                availability: finish_dt,
            })
        } else if epoch.state.is_waiting_initialisation() {
            Err(CredentialProxyError::UninitialisedDkg)
        } else {
            Err(CredentialProxyError::UnknownEcashFailure)
        }
    }

    pub async fn deposit_amount(&self, client: &ChainClient) -> Result<Coin, CredentialProxyError> {
        self.required_deposit_cache.get_or_update(client).await
    }

    pub async fn ecash_clients(
        &self,
        client: &ChainClient,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, Vec<EcashApiClient>>, CredentialProxyError> {
        self.epoch_clients
            .get_or_init(epoch_id, || async {
                Ok(client
                    .query_chain()
                    .await
                    .get_all_verification_key_shares(epoch_id)
                    .await?
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<anyhow::Result<Vec<_>, EcashApiError>>()?)
            })
            .await
    }

    pub async fn current_epoch(&self, client: &ChainClient) -> Result<Epoch, CredentialProxyError> {
        let read_guard = self.cached_epoch.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.current_epoch);
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.cached_epoch.write().await;
        let epoch = client.query_chain().await.get_current_epoch().await?;

        write_guard.update(epoch);
        Ok(epoch)
    }

    pub async fn current_epoch_id(
        &self,
        client: &ChainClient,
    ) -> Result<EpochId, CredentialProxyError> {
        let read_guard = self.cached_epoch.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.current_epoch.epoch_id);
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.cached_epoch.write().await;
        let epoch = client.query_chain().await.get_current_epoch().await?;

        write_guard.update(epoch);
        Ok(epoch.epoch_id)
    }

    pub async fn master_verification_key<S>(
        &self,
        client: &ChainClient,
        storage: &S,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>, CredentialProxyError>
    where
        S: GlobalEcashDataCache,
    {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.current_epoch_id(client).await?,
        };

        self.master_verification_key
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(stored) = storage.get_master_verification_key(epoch_id).await? {
                    return Ok(stored.key);
                }

                info!("attempting to establish master verification key for epoch {epoch_id}...");

                // 2. perform actual aggregation
                let all_apis = self.ecash_clients(client, epoch_id).await?;
                let threshold = self.ecash_threshold(client, epoch_id).await?;

                if all_apis.len() < threshold as usize {
                    return Err(CredentialProxyError::InsufficientNumberOfSigners {
                        threshold,
                        available: all_apis.len(),
                    });
                }

                let master_key = nym_credentials::aggregate_verification_keys(&all_apis)?;

                let epoch = EpochVerificationKey {
                    epoch_id,
                    key: master_key,
                };

                // 3. save the key in the storage for when we reboot
                storage.insert_master_verification_key(&epoch).await?;

                Ok(epoch.key)
            })
            .await
    }

    pub async fn master_coin_index_signatures<S>(
        &self,
        client: &ChainClient,
        storage: &S,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, AggregatedCoinIndicesSignatures>, CredentialProxyError>
    where
        S: GlobalEcashDataCache,
    {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.current_epoch_id(client).await?,
        };

        self.coin_index_signatures
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(master_sigs) =
                    storage.get_master_coin_index_signatures(epoch_id).await?
                {
                    return Ok(master_sigs);
                }

                info!(
                    "attempting to establish master coin index signatures for epoch {epoch_id}..."
                );

                // 2. go around APIs and attempt to aggregate the data
                let master_vk = self
                    .master_verification_key(client, storage, Some(epoch_id))
                    .await?;
                let all_apis = self.ecash_clients(client, epoch_id).await?;
                let threshold = self.ecash_threshold(client, epoch_id).await?;

                let get_partial_signatures = |api: EcashApiClient| async {
                    // move the api into the closure
                    let api = api;
                    let node_index = api.node_id;
                    let partial_vk = api.verification_key;

                    let partial = api
                        .api_client
                        .partial_coin_indices_signatures(Some(epoch_id))
                        .await?
                        .signatures;
                    Ok(CoinIndexSignatureShare {
                        index: node_index,
                        key: partial_vk,
                        signatures: partial,
                    })
                };

                let shares =
                    query_all_threshold_apis(all_apis.clone(), threshold, get_partial_signatures)
                        .await?;

                let aggregated = aggregate_annotated_indices_signatures(
                    nym_credentials_interface::ecash_parameters(),
                    &master_vk,
                    &shares,
                )?;

                let sigs = AggregatedCoinIndicesSignatures {
                    epoch_id,
                    signatures: aggregated,
                };

                // 3. save the signatures in the storage for when we reboot
                storage.insert_master_coin_index_signatures(&sigs).await?;

                Ok(sigs)
            })
            .await
    }

    pub async fn master_expiration_date_signatures<S>(
        &self,
        client: &ChainClient,
        storage: &S,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<'_, AggregatedExpirationDateSignatures>, CredentialProxyError>
    where
        S: GlobalEcashDataCache,
    {
        self
            .expiration_date_signatures
            .get_or_init((epoch_id, expiration_date), || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(master_sigs) = storage
                    .get_master_expiration_date_signatures(expiration_date, epoch_id)
                    .await?
                {
                    return Ok(master_sigs);
                }


                info!(
                    "attempting to establish master expiration date signatures for {expiration_date} and epoch {epoch_id}..."
                );

                // 3. go around APIs and attempt to aggregate the data
                let epoch_id = self.current_epoch_id(client).await?;
                let master_vk = self.master_verification_key(client, storage, Some(epoch_id)).await?;
                let all_apis = self.ecash_clients(client, epoch_id).await?;
                let threshold = self.ecash_threshold(client, epoch_id).await?;

                let get_partial_signatures = |api: EcashApiClient| async {
                    // move the api into the closure
                    let api = api;
                    let node_index = api.node_id;
                    let partial_vk = api.verification_key;

                    let partial = api
                        .api_client
                        .partial_expiration_date_signatures(Some(expiration_date), Some(epoch_id))
                        .await?
                        .signatures;
                    Ok(ExpirationDateSignatureShare {
                        index: node_index,
                        key: partial_vk,
                        signatures: partial,
                    })
                };

                let shares =
                    query_all_threshold_apis(all_apis.clone(), threshold, get_partial_signatures)
                        .await?;

                let aggregated = aggregate_annotated_expiration_signatures(
                    &master_vk,
                    expiration_date.ecash_unix_timestamp(),
                    &shares,
                )?;

                let sigs = AggregatedExpirationDateSignatures {
                    epoch_id,
                    expiration_date,
                    signatures: aggregated,
                };

                // 4. save the signatures in the storage for when we reboot
                storage
                    .insert_master_expiration_date_signatures(&sigs)
                    .await?;

                Ok(sigs)
            })
            .await
    }

    pub async fn ecash_threshold(
        &self,
        client: &ChainClient,
        epoch_id: EpochId,
    ) -> Result<u64, CredentialProxyError> {
        self.threshold_values
            .get_or_init(epoch_id, || async {
                if let Some(threshold) = client
                    .query_chain()
                    .await
                    .get_epoch_threshold(epoch_id)
                    .await?
                {
                    Ok(threshold)
                } else {
                    Err(CredentialProxyError::UnavailableThreshold { epoch_id })
                }
            })
            .await
            .map(|t| *t)
    }
}
