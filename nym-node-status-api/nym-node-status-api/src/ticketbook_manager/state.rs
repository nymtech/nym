// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::Storage;
use crate::ticketbook_manager::storage::TicketbookManagerStorage;
use nym_credential_proxy_lib::error::CredentialProxyError;
use nym_credential_proxy_lib::quorum_checker::QuorumState;
use nym_credential_proxy_lib::shared_state::ecash_state::{EcashState, VerificationKeyAuth};
use nym_credential_proxy_lib::shared_state::nyxd_client::ChainClient;
use nym_credential_proxy_lib::shared_state::required_deposit_cache::RequiredDepositCache;
use nym_credential_proxy_lib::storage::traits::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_ecash_time::{ecash_default_expiration_date, ecash_today};
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::Epoch;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::EcashApiClient;
use std::sync::Arc;
use time::Date;
use tokio::sync::RwLockReadGuard;

#[derive(Clone)]
pub(crate) struct TicketbookManagerState {
    storage: TicketbookManagerStorage,
    client: ChainClient,
    ecash_state: Arc<EcashState>,
}

impl TicketbookManagerState {
    pub fn new(storage: Storage, quorum_state: QuorumState, client: ChainClient) -> Self {
        let state = TicketbookManagerState {
            storage: storage.into(),
            client,
            ecash_state: Arc::new(EcashState::new(
                RequiredDepositCache::default(),
                quorum_state,
            )),
        };
        state
    }

    pub fn ecash_state(&self) -> &EcashState {
        &self.ecash_state
    }

    pub fn client(&self) -> &ChainClient {
        &self.client
    }

    pub fn storage(&self) -> &TicketbookManagerStorage {
        &self.storage
    }

    pub async fn build_initial_cache(&self) -> Result<(), CredentialProxyError> {
        let default_expiration = ecash_default_expiration_date();

        let epoch_id = self.current_epoch_id().await?;
        let _ = self.deposit_amount().await?;
        let _ = self.master_verification_key(Some(epoch_id)).await?;
        let _ = self.ecash_threshold(epoch_id).await?;
        let _ = self.ecash_clients(epoch_id).await?;
        let _ = self.master_coin_index_signatures(Some(epoch_id)).await?;
        let _ = self
            .master_expiration_date_signatures(epoch_id, default_expiration)
            .await?;

        Ok(())
    }

    pub async fn deposit_amount(&self) -> Result<Coin, CredentialProxyError> {
        self.ecash_state
            .required_deposit_cache
            .get_or_update(&self.client)
            .await
    }

    pub async fn ensure_credentials_issuable(&self) -> Result<(), CredentialProxyError> {
        self.ecash_state()
            .ensure_credentials_issuable(self.client())
            .await
    }

    pub async fn current_epoch_id(&self) -> Result<EpochId, CredentialProxyError> {
        self.ecash_state().current_epoch_id(self.client()).await
    }

    pub async fn current_epoch(&self) -> Result<Epoch, CredentialProxyError> {
        self.ecash_state().current_epoch(self.client()).await
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
