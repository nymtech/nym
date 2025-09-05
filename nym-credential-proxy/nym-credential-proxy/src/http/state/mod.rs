// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::types::RequestError;
use crate::webhook::ZkNymWebHookConfig;
use axum::http::StatusCode;
use nym_compact_ecash::PublicKeyUser;
use nym_credential_proxy_lib::deposits_buffer::{BufferedDeposit, DepositsBuffer};
use nym_credential_proxy_lib::error::CredentialProxyError;
use nym_credential_proxy_lib::quorum_checker::QuorumState;
use nym_credential_proxy_lib::shared_state::ecash_state::EcashState;
use nym_credential_proxy_lib::shared_state::nyxd_client::ChainClient;
use nym_credential_proxy_lib::shared_state::required_deposit_cache::RequiredDepositCache;
use nym_credential_proxy_lib::shared_state::CredentialProxyState;
use nym_credential_proxy_lib::storage::CredentialProxyStorage;
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    AggregatedCoinIndicesSignaturesResponse, AggregatedExpirationDateSignaturesResponse,
    MasterVerificationKeyResponse,
};
use nym_credentials::ecash::utils::ecash_today;
use nym_credentials::{AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures};
use nym_credentials_interface::VerificationKeyAuth;
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, EcashApiClient};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tokio::sync::RwLockReadGuard;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, warn};
use uuid::Uuid;

// currently we need to hold our keypair so that we could request a freepass credential
#[derive(Clone)]
pub struct ApiState {
    inner: Arc<ApiStateInner>,
}

struct ApiStateInner {
    shared_proxy_state: CredentialProxyState,

    zk_nym_web_hook_config: ZkNymWebHookConfig,

    task_tracker: TaskTracker,

    cancellation_token: CancellationToken,
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
            inner: Arc::new(ApiStateInner {
                shared_proxy_state: CredentialProxyState::new(
                    storage,
                    client,
                    deposits_buffer,
                    EcashState::new(required_deposit_cache, quorum_state),
                ),
                zk_nym_web_hook_config,
                task_tracker: TaskTracker::new(),
                cancellation_token,
            }),
        };

        // since this is startup,
        // might as well do all the needed network queries to establish needed global signatures
        // if we don't already have them
        state.build_initial_cache().await?;

        Ok(state)
    }

    fn shared_state(&self) -> &CredentialProxyState {
        &self.inner.shared_proxy_state
    }

    async fn build_initial_cache(&self) -> Result<(), CredentialProxyError> {
        let today = ecash_today().date();

        let epoch_id = self.shared_state().current_epoch_id().await?;
        let _ = self.deposit_amount().await?;
        let _ = self
            .shared_state()
            .master_verification_key(Some(epoch_id))
            .await?;
        let _ = self.shared_state().ecash_threshold(epoch_id).await?;
        let _ = self.shared_state().ecash_clients(epoch_id).await?;
        let _ = self
            .shared_state()
            .master_coin_index_signatures(Some(epoch_id))
            .await?;
        let _ = self
            .shared_state()
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
        self.shared_state()
            .deposits_buffer()
            .wait_for_shutdown()
            .await;
        self.inner.task_tracker.wait().await
    }

    pub(crate) fn zk_nym_web_hook(&self) -> &ZkNymWebHookConfig {
        &self.inner.zk_nym_web_hook_config
    }

    pub(crate) fn quorum_available(&self) -> bool {
        self.shared_state().ecash_state().quorum_state.available()
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
        self.shared_state().storage()
    }

    pub async fn deposit_amount(&self) -> Result<Coin, CredentialProxyError> {
        self.shared_state()
            .ecash_state()
            .required_deposit_cache
            .get_or_update(self.shared_state().client())
            .await
    }

    async fn current_epoch(&self) -> Result<Epoch, CredentialProxyError> {
        let read_guard = self.shared_state().ecash_state().cached_epoch.read().await;
        if read_guard.is_valid() {
            return Ok(read_guard.current_epoch);
        }

        // update cache
        drop(read_guard);
        let mut write_guard = self.shared_state().ecash_state().cached_epoch.write().await;
        let epoch = self.query_chain().await.get_current_epoch().await?;

        write_guard.update(epoch);
        Ok(epoch)
    }

    pub(crate) async fn current_epoch_id(&self) -> Result<EpochId, CredentialProxyError> {
        self.shared_state().current_epoch_id().await
    }

    pub(crate) async fn query_chain(&self) -> RwLockReadGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.shared_state().client().query_chain().await
    }

    pub(crate) async fn get_deposit(
        &self,
        request_uuid: Uuid,
        requested_on: OffsetDateTime,
        client_pubkey: PublicKeyUser,
    ) -> Result<BufferedDeposit, CredentialProxyError> {
        let start = Instant::now();
        let deposit = self
            .shared_state()
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

    pub(crate) async fn insert_deposit_usage_error(&self, deposit_id: DepositId, error: String) {
        if let Err(err) = self
            .shared_state()
            .storage()
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
        self.shared_state()
            .global_data(
                include_master_verification_key,
                include_expiration_date_signatures,
                include_coin_index_signatures,
                epoch_id,
                expiration_date,
            )
            .await
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

    pub async fn master_verification_key(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, VerificationKeyAuth>, CredentialProxyError> {
        self.shared_state().master_verification_key(epoch_id).await
    }

    pub async fn master_coin_index_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, AggregatedCoinIndicesSignatures>, CredentialProxyError> {
        self.shared_state()
            .master_coin_index_signatures(epoch_id)
            .await
    }

    pub async fn master_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<'_, AggregatedExpirationDateSignatures>, CredentialProxyError> {
        self.shared_state()
            .master_expiration_date_signatures(epoch_id, expiration_date)
            .await
    }

    pub async fn ecash_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, Vec<EcashApiClient>>, CredentialProxyError> {
        self.shared_state().ecash_clients(epoch_id).await
    }

    pub async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<u64, CredentialProxyError> {
        self.shared_state().ecash_threshold(epoch_id).await
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
}
