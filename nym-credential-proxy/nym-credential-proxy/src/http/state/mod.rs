// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::deposits_buffer::helpers::BufferedDeposit;
use crate::deposits_buffer::DepositsBuffer;
use crate::error::CredentialProxyError;
use crate::helpers::LockTimer;
use crate::http::state::required_deposit_cache::RequiredDepositCache;
use crate::http::types::RequestError;
use crate::nym_api_helpers::{
    ensure_sane_expiration_date, query_all_threshold_apis, CachedEpoch, CachedImmutableEpochItem,
    CachedImmutableItems,
};
use crate::quorum_checker::QuorumState;
use crate::storage::CredentialProxyStorage;
use crate::webhook::ZkNymWebHookConfig;
use axum::http::StatusCode;
use bip39::Mnemonic;
use nym_compact_ecash::scheme::coin_indices_signatures::{
    aggregate_annotated_indices_signatures, CoinIndexSignatureShare,
};
use nym_compact_ecash::scheme::expiration_date_signatures::{
    aggregate_annotated_expiration_signatures, ExpirationDateSignatureShare,
};
use nym_compact_ecash::{Base58, PublicKeyUser};
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    AggregatedCoinIndicesSignaturesResponse, AggregatedExpirationDateSignaturesResponse,
    MasterVerificationKeyResponse,
};
use nym_credentials::ecash::utils::{ecash_today, EcashTime};
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_credentials_interface::VerificationKeyAuth;
use nym_ecash_contract_common::deposit::DepositId;
use nym_ecash_contract_common::msg::ExecuteMsg;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, NymContractsProvider, PagedDkgQueryClient,
};
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::{Coin, CosmWasmClient, NyxdClient};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient, EcashApiClient};
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

pub(crate) mod required_deposit_cache;

// currently we need to hold our keypair so that we could request a freepass credential
#[derive(Clone)]
pub struct ApiState {
    inner: Arc<CredentialProxyStateInner>,
}

// a lot of functionalities, mostly to do with caching and storage is just copy-pasted from nym-api,
// since we have to do more or less the same work
impl ApiState {
    pub(crate) async fn new(
        storage: CredentialProxyStorage,
        quorum_state: QuorumState,
        zk_nym_web_hook_config: ZkNymWebHookConfig,
        client: ChainClient,
        deposits_buffer: DepositsBuffer,
        required_deposit_cache: RequiredDepositCache,
        cancellation_token: CancellationToken,
    ) -> Result<Self, CredentialProxyError> {
        let state = ApiState {
            inner: Arc::new(CredentialProxyStateInner {
                storage,
                client,
                ecash_state: EcashState {
                    required_deposit_cache,
                    quorum_state,
                    cached_epoch: Default::default(),
                    master_verification_key: Default::default(),
                    threshold_values: Default::default(),
                    epoch_clients: Default::default(),
                    coin_index_signatures: Default::default(),
                    expiration_date_signatures: Default::default(),
                },
                zk_nym_web_hook_config,
                task_tracker: TaskTracker::new(),
                deposits_buffer,
                cancellation_token,
            }),
        };

        // since this is startup,
        // might as well do all the needed network queries to establish needed global signatures
        // if we don't already have them
        state.build_initial_cache().await?;

        Ok(state)
    }

    async fn build_initial_cache(&self) -> Result<(), CredentialProxyError> {
        let today = ecash_today().date();

        let epoch_id = self.current_epoch_id().await?;
        let _ = self.deposit_amount().await?;
        let _ = self.master_verification_key(Some(epoch_id)).await?;
        let _ = self.ecash_threshold(epoch_id).await?;
        let _ = self.ecash_clients(epoch_id).await?;
        let _ = self.master_coin_index_signatures(Some(epoch_id)).await?;
        let _ = self
            .master_expiration_date_signatures(epoch_id, today)
            .await?;

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
        self.inner.deposits_buffer.wait_for_shutdown().await;
        self.inner.task_tracker.wait().await
    }

    pub(crate) fn zk_nym_web_hook(&self) -> &ZkNymWebHookConfig {
        &self.inner.zk_nym_web_hook_config
    }

    pub(crate) fn quorum_available(&self) -> bool {
        self.inner.ecash_state.quorum_state.available()
    }

    async fn ensure_credentials_issuable(&self) -> Result<(), CredentialProxyError> {
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

    pub(crate) fn storage(&self) -> &CredentialProxyStorage {
        &self.inner.storage
    }

    pub async fn deposit_amount(&self) -> Result<Coin, CredentialProxyError> {
        self.inner
            .ecash_state
            .required_deposit_cache
            .get_or_update(&self.inner.client)
            .await
    }

    async fn current_epoch(&self) -> Result<Epoch, CredentialProxyError> {
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

    pub(crate) async fn query_chain(&self) -> RwLockReadGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.inner.client.query_chain().await
    }

    pub(crate) async fn get_deposit(
        &self,
        request_uuid: Uuid,
        requested_on: OffsetDateTime,
        client_pubkey: PublicKeyUser,
    ) -> Result<BufferedDeposit, CredentialProxyError> {
        let start = Instant::now();
        let deposit = self
            .inner
            .deposits_buffer
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

    pub(crate) async fn insert_deposit_usage_error(&self, deposit_id: DepositId, error: String) {
        if let Err(err) = self
            .inner
            .storage
            .insert_deposit_usage_error(deposit_id, error)
            .await
        {
            error!("failed to insert information about deposit (id: {deposit_id}) usage failure: {err}")
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
        CredentialProxyError,
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

    pub(crate) async fn ecash_threshold(
        &self,
        epoch_id: EpochId,
    ) -> Result<u64, CredentialProxyError> {
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

    pub(crate) async fn master_coin_index_signatures(
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

    pub(crate) async fn master_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<'_, AggregatedExpirationDateSignatures>, CredentialProxyError> {
        self.inner
            .ecash_state
            .expiration_date_signatures
            .get_or_init((epoch_id, expiration_date), || async {
                // 1. sanity check to see if the expiration_date is not nonsense
                ensure_sane_expiration_date(expiration_date)?;

                // 2. check the storage
                if let Some(master_sigs) = self
                    .inner
                    .storage
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
                self.inner
                    .storage
                    .insert_master_expiration_date_signatures(&sigs)
                    .await?;

                Ok(sigs)
            })
            .await
    }
}

#[derive(Clone)]
pub struct ChainClient(Arc<RwLock<DirectSigningHttpRpcNyxdClient>>);

impl ChainClient {
    pub fn new(mnemonic: Mnemonic) -> Result<Self, CredentialProxyError> {
        let network_details = nym_network_defaults::NymNetworkDetails::new_from_env();
        let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        let nyxd_url = network_details
            .endpoints
            .first()
            .ok_or_else(|| CredentialProxyError::NoNyxEndpointsAvailable)?
            .nyxd_url
            .as_str();

        let client = NyxdClient::connect_with_mnemonic(client_config, nyxd_url, mnemonic)?;

        if client.ecash_contract_address().is_none() {
            return Err(CredentialProxyError::UnavailableEcashContract);
        }

        if client.dkg_contract_address().is_none() {
            return Err(CredentialProxyError::UnavailableDKGContract);
        }

        Ok(ChainClient(Arc::new(RwLock::new(client))))
    }

    pub(crate) async fn query_chain(&self) -> ChainReadPermit<'_> {
        let _acquire_timer = LockTimer::new("acquire chain query permit");
        self.0.read().await
    }

    pub(crate) async fn start_chain_tx(&self) -> ChainWritePermit<'_> {
        let _acquire_timer = LockTimer::new("acquire exclusive chain write permit");

        ChainWritePermit {
            lock_timer: LockTimer::new("exclusive chain access permit"),
            inner: self.0.write().await,
        }
    }
}

struct CredentialProxyStateInner {
    storage: CredentialProxyStorage,

    client: ChainClient,

    deposits_buffer: DepositsBuffer,

    zk_nym_web_hook_config: ZkNymWebHookConfig,

    ecash_state: EcashState,

    task_tracker: TaskTracker,

    cancellation_token: CancellationToken,
}

pub(crate) struct EcashState {
    pub(crate) required_deposit_cache: RequiredDepositCache,

    pub(crate) quorum_state: QuorumState,

    pub(crate) cached_epoch: RwLock<CachedEpoch>,

    pub(crate) master_verification_key: CachedImmutableEpochItem<VerificationKeyAuth>,

    pub(crate) threshold_values: CachedImmutableEpochItem<u64>,

    pub(crate) epoch_clients: CachedImmutableEpochItem<Vec<EcashApiClient>>,

    pub(crate) coin_index_signatures: CachedImmutableEpochItem<AggregatedCoinIndicesSignatures>,

    pub(crate) expiration_date_signatures:
        CachedImmutableItems<(EpochId, Date), AggregatedExpirationDateSignatures>,
}

pub(crate) type ChainReadPermit<'a> = RwLockReadGuard<'a, DirectSigningHttpRpcNyxdClient>;

// explicitly wrap the WriteGuard for extra information regarding time taken
pub(crate) struct ChainWritePermit<'a> {
    // it's not really dead, we only care about it being dropped
    #[allow(dead_code)]
    lock_timer: LockTimer,
    inner: RwLockWriteGuard<'a, DirectSigningHttpRpcNyxdClient>,
}

impl ChainWritePermit<'_> {
    #[instrument(skip(self, short_sha, info), err(Display))]
    pub(crate) async fn make_deposits(
        self,
        short_sha: &'static str,
        info: Vec<(String, Coin)>,
    ) -> Result<ExecuteResult, CredentialProxyError> {
        let address = self.inner.address();
        let starting_sequence = self.inner.get_sequence(&address).await?.sequence;

        let deposits = info.len();

        let ecash_contract = self
            .inner
            .ecash_contract_address()
            .ok_or(CredentialProxyError::UnavailableEcashContract)?;
        let deposit_messages = info
            .into_iter()
            .map(|(identity_key, amount)| {
                (
                    ExecuteMsg::DepositTicketBookFunds { identity_key },
                    vec![amount],
                )
            })
            .collect::<Vec<_>>();

        let res = self
            .inner
            .execute_multiple(
                ecash_contract,
                deposit_messages,
                None,
                format!("cp-{short_sha}: performing {deposits} deposits"),
            )
            .await?;

        loop {
            let updated_sequence = self.inner.get_sequence(&address).await?.sequence;

            if updated_sequence > starting_sequence {
                break;
            }
            warn!("wrong sequence number... waiting before releasing chain lock");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(res)
    }
}

impl Deref for ChainWritePermit<'_> {
    type Target = DirectSigningHttpRpcNyxdClient;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}
