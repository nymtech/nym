// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::deposits_buffer::{BufferedDeposit, DepositsBuffer};
use crate::error::CredentialProxyError;
use crate::nym_api_helpers::{ensure_sane_expiration_date, query_all_threshold_apis};
use crate::shared_state::ecash_state::EcashState;
use crate::shared_state::nyxd_client::ChainClient;
use crate::storage::CredentialProxyStorage;
use nym_compact_ecash::scheme::coin_indices_signatures::{
    aggregate_annotated_indices_signatures, CoinIndexSignatureShare,
};
use nym_compact_ecash::scheme::expiration_date_signatures::{
    aggregate_annotated_expiration_signatures, ExpirationDateSignatureShare,
};
use nym_compact_ecash::{Base58, PublicKeyUser, VerificationKeyAuth};
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    AggregatedCoinIndicesSignaturesResponse, AggregatedExpirationDateSignaturesResponse,
    GlobalDataParams, MasterVerificationKeyResponse,
};
use nym_credentials::ecash::utils::EcashTime;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use nym_validator_client::nyxd::Coin;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, EcashApiClient};
use std::sync::Arc;
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tokio::sync::RwLockReadGuard;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub mod ecash_state;
pub mod nyxd_client;
pub mod required_deposit_cache;

#[derive(Clone)]
pub struct CredentialProxyState {
    inner: Arc<CredentialProxyStateInner>,
}

impl CredentialProxyState {
    pub fn new(
        storage: CredentialProxyStorage,
        client: ChainClient,
        deposits_buffer: DepositsBuffer,
        ecash_state: EcashState,
    ) -> Self {
        CredentialProxyState {
            inner: Arc::new(CredentialProxyStateInner {
                storage,
                client,
                deposits_buffer,
                ecash_state,
            }),
        }
    }

    pub fn storage(&self) -> &CredentialProxyStorage {
        &self.inner.storage
    }

    pub async fn deposit_amount(&self) -> Result<Coin, CredentialProxyError> {
        self.ecash_state()
            .required_deposit_cache
            .get_or_update(self.client())
            .await
    }

    pub fn client(&self) -> &ChainClient {
        &self.inner.client
    }

    pub fn deposits_buffer(&self) -> &DepositsBuffer {
        &self.inner.deposits_buffer
    }

    pub fn ecash_state(&self) -> &EcashState {
        &self.inner.ecash_state
    }

    pub(crate) async fn query_chain(&self) -> RwLockReadGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.inner.client.query_chain().await
    }

    pub async fn ensure_credentials_issuable(&self) -> Result<(), CredentialProxyError> {
        let epoch = self.current_epoch().await?;

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

    pub async fn get_deposit(
        &self,
        request_uuid: Uuid,
        requested_on: OffsetDateTime,
        client_pubkey: PublicKeyUser,
    ) -> Result<BufferedDeposit, CredentialProxyError> {
        let start = Instant::now();
        let deposit = self
            .deposits_buffer()
            .get_valid_deposit(request_uuid, requested_on, client_pubkey)
            .await;

        let time_taken = start.elapsed();
        let formatted = humantime::format_duration(time_taken);
        if time_taken > Duration::from_secs(10) {
            warn!("attempting to get buffered deposit took {formatted}. perhaps the buffer is too small or the process/chain is overloaded?")
        } else {
            debug!("attempting to get buffered deposit took {formatted}")
        };

        deposit
    }

    pub async fn insert_deposit_usage_error(&self, deposit_id: DepositId, error: String) {
        if let Err(err) = self
            .storage()
            .insert_deposit_usage_error(deposit_id, error)
            .await
        {
            error!("failed to insert information about deposit (id: {deposit_id}) usage failure: {err}")
        }
    }

    pub async fn current_epoch_id(&self) -> Result<EpochId, CredentialProxyError> {
        let read_guard = self.inner.ecash_state.cached_epoch.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.current_epoch.epoch_id);
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.inner.ecash_state.cached_epoch.write().await;
        let epoch = self.query_chain().await.get_current_epoch().await?;

        write_guard.update(epoch);
        Ok(epoch.epoch_id)
    }

    pub async fn current_epoch(&self) -> Result<Epoch, CredentialProxyError> {
        let read_guard = self.ecash_state().cached_epoch.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.current_epoch);
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.ecash_state().cached_epoch.write().await;
        let epoch = self.query_chain().await.get_current_epoch().await?;

        write_guard.update(epoch);
        Ok(epoch)
    }

    pub async fn global_data(
        &self,
        global_data: GlobalDataParams,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<
        (
            Option<MasterVerificationKeyResponse>,
            Option<AggregatedExpirationDateSignaturesResponse>,
            Option<AggregatedCoinIndicesSignaturesResponse>,
        ),
        CredentialProxyError,
    > {
        let master_verification_key = if global_data.include_master_verification_key {
            debug!("including master verification key in the response");
            Some(
                self.master_verification_key(Some(epoch_id))
                    .await
                    .map(|key| MasterVerificationKeyResponse {
                        epoch_id,
                        bs58_encoded_key: key.to_bs58(),
                    })
                    .inspect_err(|err| warn!("request failure: {err}"))?,
            )
        } else {
            None
        };

        let aggregated_expiration_date_signatures =
            if global_data.include_expiration_date_signatures {
                debug!("including expiration date signatures in the response");
                Some(
                    self.master_expiration_date_signatures(epoch_id, expiration_date)
                        .await
                        .map(|signatures| AggregatedExpirationDateSignaturesResponse {
                            signatures: signatures.clone(),
                        })
                        .inspect_err(|err| warn!("request failure: {err}"))?,
                )
            } else {
                None
            };

        let aggregated_coin_index_signatures = if global_data.include_coin_index_signatures {
            debug!("including coin index signatures in the response");
            Some(
                self.master_coin_index_signatures(Some(epoch_id))
                    .await
                    .map(|signatures| AggregatedCoinIndicesSignaturesResponse {
                        signatures: signatures.clone(),
                    })
                    .inspect_err(|err| warn!("request failure: {err}"))?,
            )
        } else {
            None
        };

        Ok((
            master_verification_key,
            aggregated_expiration_date_signatures,
            aggregated_coin_index_signatures,
        ))
    }

    pub async fn master_verification_key(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>, CredentialProxyError> {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.current_epoch_id().await?,
        };

        self.inner
            .ecash_state
            .master_verification_key
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(stored) = self
                    .inner
                    .storage
                    .get_master_verification_key(epoch_id)
                    .await?
                {
                    return Ok(stored.key);
                }

                info!("attempting to establish master verification key for epoch {epoch_id}...");

                // 2. perform actual aggregation
                let all_apis = self.ecash_clients(epoch_id).await?;
                let threshold = self.ecash_threshold(epoch_id).await?;

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
                self.inner
                    .storage
                    .insert_master_verification_key(&epoch)
                    .await?;

                Ok(epoch.key)
            })
            .await
    }

    pub async fn master_coin_index_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, AggregatedCoinIndicesSignatures>, CredentialProxyError> {
        let epoch_id = match epoch_id {
            Some(id) => id,
            None => self.current_epoch_id().await?,
        };

        self.inner
            .ecash_state
            .coin_index_signatures
            .get_or_init(epoch_id, || async {
                // 1. check the storage
                if let Some(master_sigs) = self
                    .inner
                    .storage
                    .get_master_coin_index_signatures(epoch_id)
                    .await?
                {
                    return Ok(master_sigs);
                }

                info!(
                    "attempting to establish master coin index signatures for epoch {epoch_id}..."
                );

                // 2. go around APIs and attempt to aggregate the data
                let master_vk = self.master_verification_key(Some(epoch_id)).await?;
                let all_apis = self.ecash_clients(epoch_id).await?;
                let threshold = self.ecash_threshold(epoch_id).await?;

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
                self.inner
                    .storage
                    .insert_master_coin_index_signatures(&sigs)
                    .await?;

                Ok(sigs)
            })
            .await
    }

    pub async fn master_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<'_, AggregatedExpirationDateSignatures>, CredentialProxyError> {
        self.inner.ecash_state
            .expiration_date_signatures
            .get_or_init((epoch_id, expiration_date), || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(master_sigs) = self
                    .storage()
                    .get_master_expiration_date_signatures(expiration_date, epoch_id)
                    .await?
                {
                    return Ok(master_sigs);
                }


                info!(
                    "attempting to establish master expiration date signatures for {expiration_date} and epoch {epoch_id}..."
                );

                // 3. go around APIs and attempt to aggregate the data
                let epoch_id = self.current_epoch_id().await?;
                let master_vk = self.master_verification_key(Some(epoch_id)).await?;
                let all_apis = self.ecash_clients(epoch_id).await?;
                let threshold = self.ecash_threshold(epoch_id).await?;

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
                self.inner.storage
                    .insert_master_expiration_date_signatures(&sigs)
                    .await?;

                Ok(sigs)
            })
            .await
    }

    pub async fn ecash_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, Vec<EcashApiClient>>, CredentialProxyError> {
        self.inner
            .ecash_state
            .epoch_clients
            .get_or_init(epoch_id, || async {
                Ok(self
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

    pub async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<u64, CredentialProxyError> {
        self.inner
            .ecash_state
            .threshold_values
            .get_or_init(epoch_id, || async {
                if let Some(threshold) = self
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

struct CredentialProxyStateInner {
    storage: CredentialProxyStorage,

    client: ChainClient,

    deposits_buffer: DepositsBuffer,

    ecash_state: EcashState,
}
