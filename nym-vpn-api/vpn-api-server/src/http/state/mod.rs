// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::VpnApiError;
use crate::helpers::LockTimer;
use crate::http::types::RequestError;
use crate::nym_api_helpers::{
    ensure_sane_expiration_date, query_all_threshold_apis, CachedEpoch, CachedImmutableEpochItem,
    CachedImmutableItems,
};
use crate::storage::VpnApiStorage;
use crate::webhook::ZkNymWebHookConfig;
use axum::http::StatusCode;
use bip39::Mnemonic;
use nym_compact_ecash::scheme::coin_indices_signatures::{
    aggregate_annotated_indices_signatures, CoinIndexSignatureShare,
};
use nym_compact_ecash::scheme::expiration_date_signatures::{
    aggregate_annotated_expiration_signatures, ExpirationDateSignatureShare,
};
use nym_compact_ecash::Base58;
use nym_credentials::ecash::utils::{ecash_today, EcashTime};
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_credentials_interface::VerificationKeyAuth;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, EcashQueryClient, NymContractsProvider, PagedDkgQueryClient,
};
use nym_validator_client::nyxd::{Coin, NyxdClient};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient, EcashApiClient};
use nym_vpn_api_requests::api::v1::ticketbook::models::{
    AggregatedCoinIndicesSignaturesResponse, AggregatedExpirationDateSignaturesResponse,
    MasterVerificationKeyResponse,
};
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, info, warn};
use uuid::Uuid;

// currently we need to hold our keypair so that we could request a freepass credential
#[derive(Clone)]
pub struct ApiState {
    inner: Arc<ApiStateInner>,
}

// a lot of functionalities, mostly to do with caching and storage is just copy-pasted from nym-api,
// since we have to do more or less the same work
impl ApiState {
    pub async fn new(
        storage: VpnApiStorage,
        zk_nym_web_hook_config: ZkNymWebHookConfig,
        mnemonic: Mnemonic,
    ) -> Result<Self, VpnApiError> {
        let network_details = nym_network_defaults::NymNetworkDetails::new_from_env();
        let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        let nyxd_url = network_details
            .endpoints
            .first()
            .ok_or_else(|| VpnApiError::NoNyxEndpointsAvailable)?
            .nyxd_url
            .as_str();

        let client = NyxdClient::connect_with_mnemonic(client_config, nyxd_url, mnemonic)?;

        if client.ecash_contract_address().is_none() {
            return Err(VpnApiError::UnavailableEcashContract);
        }

        if client.dkg_contract_address().is_none() {
            return Err(VpnApiError::UnavailableDKGContract);
        }

        let state = ApiState {
            inner: Arc::new(ApiStateInner {
                storage,
                client: RwLock::new(client),
                ecash_state: EcashState::default(),
                zk_nym_web_hook_config,
                task_tracker: TaskTracker::new(),
                cancellation_token: CancellationToken::new(),
            }),
        };

        // since this is startup,
        // might as well do all the needed network queries to establish needed global signatures
        // if we don't already have them
        state.build_initial_cache().await?;

        Ok(state)
    }

    async fn build_initial_cache(&self) -> Result<(), VpnApiError> {
        let today = ecash_today().date();

        let epoch_id = self.current_epoch_id().await?;
        let _ = self.deposit_amount().await?;
        let _ = self.master_verification_key(Some(epoch_id)).await?;
        let _ = self.ecash_threshold(epoch_id).await?;
        let _ = self.ecash_clients(epoch_id).await?;
        let _ = self.master_coin_index_signatures(Some(epoch_id)).await?;
        let _ = self.master_expiration_date_signatures(today).await?;

        Ok(())
    }

    pub(crate) fn try_spawn<F>(&self, task: F) -> Option<JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        // don't spawn new task if we've received cancellation token
        if self.inner.cancellation_token.is_cancelled() {
            None
        } else {
            self.inner.task_tracker.reopen();
            // TODO: later use a task queue since most requests will be blocked waiting on chain permit anyway
            let join_handle = self.inner.task_tracker.spawn(task);
            self.inner.task_tracker.close();
            Some(join_handle)
        }
    }

    pub(crate) async fn cancel_and_wait(&self) {
        self.inner.cancellation_token.cancel();
        self.inner.task_tracker.wait().await
    }

    pub(crate) fn cancellation_token(&self) -> CancellationToken {
        self.inner.cancellation_token.clone()
    }

    pub(crate) fn zk_nym_web_hook(&self) -> &ZkNymWebHookConfig {
        &self.inner.zk_nym_web_hook_config
    }

    async fn ensure_credentials_issuable(&self) -> Result<(), VpnApiError> {
        let epoch = self.current_epoch().await?;

        if epoch.state.is_final() {
            Ok(())
        } else if let Some(final_timestamp) = epoch.final_timestamp_secs() {
            // SAFETY: the timestamp values in our DKG contract should be valid timestamps,
            // otherwise it means the chain is seriously misbehaving
            #[allow(clippy::unwrap_used)]
            let finish_dt = OffsetDateTime::from_unix_timestamp(final_timestamp as i64).unwrap();

            Err(VpnApiError::CredentialsNotYetIssuable {
                availability: finish_dt,
            })
        } else if epoch.state.is_waiting_initialisation() {
            return Err(VpnApiError::UninitialisedDkg);
        } else {
            Err(VpnApiError::UnknownEcashFailure)
        }
    }

    pub(crate) fn storage(&self) -> &VpnApiStorage {
        &self.inner.storage
    }

    pub async fn deposit_amount(&self) -> Result<Coin, VpnApiError> {
        let read_guard = self.inner.ecash_state.required_deposit_cache.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.required_amount.clone());
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.inner.ecash_state.required_deposit_cache.write().await;
        let deposit_amount = self
            .query_chain()
            .await
            .get_required_deposit_amount()
            .await?;

        write_guard.update(deposit_amount.clone().into());

        Ok(deposit_amount.into())
    }

    async fn current_epoch(&self) -> Result<Epoch, VpnApiError> {
        let read_guard = self.inner.ecash_state.cached_epoch.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.current_epoch);
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.inner.ecash_state.cached_epoch.write().await;
        let epoch = self.query_chain().await.get_current_epoch().await?;

        write_guard.update(epoch);
        Ok(epoch)
    }

    pub async fn current_epoch_id(&self) -> Result<EpochId, VpnApiError> {
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

    pub(crate) async fn query_chain(&self) -> RwLockReadGuard<DirectSigningHttpRpcNyxdClient> {
        let _acquire_timer = LockTimer::new("acquire chain query permit");
        self.inner.client.read().await
    }

    pub(crate) async fn start_chain_tx(&self) -> ChainWritePermit {
        let _acquire_timer = LockTimer::new("acquire exclusive chain write permit");

        ChainWritePermit {
            lock_timer: LockTimer::new("exclusive chain access permit"),
            inner: self.inner.client.write().await,
        }
    }

    pub(crate) async fn global_data(
        &self,
        include_master_verification_key: bool,
        include_expiration_date_signatures: bool,
        include_coin_index_signatures: bool,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<
        (
            Option<MasterVerificationKeyResponse>,
            Option<AggregatedExpirationDateSignaturesResponse>,
            Option<AggregatedCoinIndicesSignaturesResponse>,
        ),
        VpnApiError,
    > {
        let master_verification_key = if include_master_verification_key {
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

        let aggregated_expiration_date_signatures = if include_expiration_date_signatures {
            debug!("including expiration date signatures in the response");
            Some(
                self.master_expiration_date_signatures(expiration_date)
                    .await
                    .map(|signatures| AggregatedExpirationDateSignaturesResponse {
                        signatures: signatures.clone(),
                    })
                    .inspect_err(|err| warn!("request failure: {err}"))?,
            )
        } else {
            None
        };

        let aggregated_coin_index_signatures = if include_coin_index_signatures {
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

    pub(crate) async fn response_global_data(
        &self,
        include_master_verification_key: bool,
        include_expiration_date_signatures: bool,
        include_coin_index_signatures: bool,
        epoch_id: EpochId,
        expiration_date: Date,
        uuid: Uuid,
    ) -> Result<
        (
            Option<MasterVerificationKeyResponse>,
            Option<AggregatedExpirationDateSignaturesResponse>,
            Option<AggregatedCoinIndicesSignaturesResponse>,
        ),
        RequestError,
    > {
        self.global_data(
            include_master_verification_key,
            include_expiration_date_signatures,
            include_coin_index_signatures,
            epoch_id,
            expiration_date,
        )
        .await
        .map_err(|err| RequestError::new_server_error(err, uuid))
    }

    pub async fn ensure_not_in_epoch_transition(
        &self,
        uuid: Option<Uuid>,
    ) -> Result<(), RequestError> {
        if let Err(err) = self.ensure_credentials_issuable().await {
            return if let Some(uuid) = uuid {
                Err(RequestError::new_with_uuid(
                    err.to_string(),
                    uuid,
                    StatusCode::SERVICE_UNAVAILABLE,
                ))
            } else {
                Err(RequestError::new(
                    err.to_string(),
                    StatusCode::SERVICE_UNAVAILABLE,
                ))
            };
        }
        Ok(())
    }

    pub(crate) async fn ecash_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<Vec<EcashApiClient>>, VpnApiError> {
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

    pub(crate) async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<u64, VpnApiError> {
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
                    Err(VpnApiError::UnavailableThreshold { epoch_id })
                }
            })
            .await
            .map(|t| *t)
    }

    pub(crate) async fn response_ecash_threshold(
        &self,
        uuid: Uuid,
        epoch_id: EpochId,
    ) -> Result<u64, RequestError> {
        self.ecash_threshold(epoch_id)
            .await
            .map_err(|err| RequestError::new_server_error(err, uuid))
    }

    pub(crate) async fn master_verification_key(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<VerificationKeyAuth>, VpnApiError> {
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
                    return Err(VpnApiError::InsufficientNumberOfSigners {
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

    pub(crate) async fn master_coin_index_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<AggregatedCoinIndicesSignatures>, VpnApiError> {
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

    pub(crate) async fn master_expiration_date_signatures(
        &self,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<AggregatedExpirationDateSignatures>, VpnApiError> {
        self.inner
            .ecash_state
            .expiration_date_signatures
            .get_or_init(expiration_date, || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(master_sigs) = self
                    .inner
                    .storage
                    .get_master_expiration_date_signatures(expiration_date)
                    .await?
                {
                    return Ok(master_sigs);
                }


                info!(
                    "attempting to establish master expiration date signatures for {expiration_date}..."
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
                        .partial_expiration_date_signatures(Some(expiration_date))
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
                self.inner
                    .storage
                    .insert_master_expiration_date_signatures(&sigs)
                    .await?;

                Ok(sigs)
            })
            .await
    }
}

struct ApiStateInner {
    storage: VpnApiStorage,

    client: RwLock<DirectSigningHttpRpcNyxdClient>,

    zk_nym_web_hook_config: ZkNymWebHookConfig,

    ecash_state: EcashState,

    task_tracker: TaskTracker,

    cancellation_token: CancellationToken,
}

pub(crate) struct CachedDeposit {
    valid_until: OffsetDateTime,
    required_amount: Coin,
}

impl CachedDeposit {
    const MAX_VALIDITY: time::Duration = time::Duration::MINUTE;

    fn is_valid(&self) -> bool {
        self.valid_until > OffsetDateTime::now_utc()
    }

    fn update(&mut self, required_amount: Coin) {
        self.valid_until = OffsetDateTime::now_utc() + Self::MAX_VALIDITY;
        self.required_amount = required_amount;
    }
}

impl Default for CachedDeposit {
    fn default() -> Self {
        CachedDeposit {
            valid_until: OffsetDateTime::UNIX_EPOCH,
            required_amount: Coin {
                amount: u128::MAX,
                denom: "unym".to_string(),
            },
        }
    }
}

#[derive(Default)]
pub(crate) struct EcashState {
    pub(crate) required_deposit_cache: RwLock<CachedDeposit>,

    pub(crate) cached_epoch: RwLock<CachedEpoch>,

    pub(crate) master_verification_key: CachedImmutableEpochItem<VerificationKeyAuth>,

    pub(crate) threshold_values: CachedImmutableEpochItem<u64>,

    pub(crate) epoch_clients: CachedImmutableEpochItem<Vec<EcashApiClient>>,

    pub(crate) coin_index_signatures: CachedImmutableEpochItem<AggregatedCoinIndicesSignatures>,

    pub(crate) expiration_date_signatures:
        CachedImmutableItems<Date, AggregatedExpirationDateSignatures>,
}

// explicitly wrap the WriteGuard for extra information regarding time taken
pub(crate) struct ChainWritePermit<'a> {
    // it's not really dead, we only care about it being dropped
    #[allow(dead_code)]
    lock_timer: LockTimer,
    inner: RwLockWriteGuard<'a, DirectSigningHttpRpcNyxdClient>,
}

impl<'a> Deref for ChainWritePermit<'a> {
    type Target = DirectSigningHttpRpcNyxdClient;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}
