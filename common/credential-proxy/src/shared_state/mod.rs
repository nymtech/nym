// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::deposits_buffer::{BufferedDeposit, DepositsBuffer};
use crate::error::CredentialProxyError;
use crate::shared_state::ecash_state::EcashState;
use crate::shared_state::nyxd_client::ChainClient;
use crate::storage::CredentialProxyStorage;

use nym_compact_ecash::{Base58, PublicKeyUser, VerificationKeyAuth};
use nym_credential_proxy_requests::api::v1::ticketbook::models::{
    AggregatedCoinIndicesSignaturesResponse, AggregatedExpirationDateSignaturesResponse,
    GlobalDataParams, MasterVerificationKeyResponse,
};
use nym_credentials::{AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures};
use nym_ecash_contract_common::deposit::DepositId;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::EcashApiClient;
use std::sync::Arc;
use std::time::Duration;
use time::{Date, OffsetDateTime};
use tokio::sync::RwLockReadGuard;
use tokio::time::Instant;
use tracing::{debug, error, warn};
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
        self.ecash_state().deposit_amount(self.client()).await
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

    pub async fn ensure_credentials_issuable(&self) -> Result<(), CredentialProxyError> {
        self.ecash_state()
            .ensure_credentials_issuable(self.client())
            .await
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
        self.ecash_state().current_epoch_id(self.client()).await
    }

    pub async fn current_epoch(&self) -> Result<Epoch, CredentialProxyError> {
        self.ecash_state().current_epoch(self.client()).await
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
        self.ecash_state()
            .master_verification_key(self.client(), self.storage(), epoch_id)
            .await
    }

    pub async fn master_coin_index_signatures(
        &self,
        epoch_id: Option<EpochId>,
    ) -> Result<RwLockReadGuard<'_, AggregatedCoinIndicesSignatures>, CredentialProxyError> {
        self.ecash_state()
            .master_coin_index_signatures(self.client(), self.storage(), epoch_id)
            .await
    }

    pub async fn master_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<RwLockReadGuard<'_, AggregatedExpirationDateSignatures>, CredentialProxyError> {
        self.ecash_state()
            .master_expiration_date_signatures(
                self.client(),
                self.storage(),
                epoch_id,
                expiration_date,
            )
            .await
    }

    pub async fn ecash_clients(
        &self,
        epoch_id: EpochId,
    ) -> Result<RwLockReadGuard<'_, Vec<EcashApiClient>>, CredentialProxyError> {
        self.ecash_state()
            .ecash_clients(self.client(), epoch_id)
            .await
    }

    pub async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<u64, CredentialProxyError> {
        self.ecash_state()
            .ecash_threshold(self.client(), epoch_id)
            .await
    }
}

struct CredentialProxyStateInner {
    storage: CredentialProxyStorage,

    client: ChainClient,

    deposits_buffer: DepositsBuffer,

    ecash_state: EcashState,
}
