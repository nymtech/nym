// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::db::Storage;
use crate::ticketbook_manager::storage::TicketbookManagerStorage;
use nym_credential_proxy_lib::error::CredentialProxyError;
use nym_credential_proxy_lib::quorum_checker::QuorumState;
use nym_credential_proxy_lib::shared_state::ecash_state::{
    EcashState, TicketType, VerificationKeyAuth,
};
use nym_credential_proxy_lib::shared_state::nyxd_client::ChainClient;
use nym_credential_proxy_lib::shared_state::required_deposit_cache::RequiredDepositCache;
use nym_credential_proxy_lib::storage::traits::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::EpochVerificationKey;
use nym_ecash_time::ecash_default_expiration_date;
use nym_node_status_client::models::AttachedTicketMaterials;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::EcashApiClient;
use std::collections::HashMap;
use std::sync::Arc;
use time::Date;
use tokio::sync::RwLockReadGuard;
use tracing::{debug, warn};

#[derive(Clone)]
pub(crate) struct TicketbookManagerState {
    buffered_ticket_types: Vec<TicketType>,
    storage: TicketbookManagerStorage,
    client: ChainClient,
    ecash_state: Arc<EcashState>,
}

impl TicketbookManagerState {
    pub fn new(
        buffered_ticket_types: Vec<TicketType>,
        storage: Storage,
        quorum_state: QuorumState,
        client: ChainClient,
    ) -> Self {
        let state = TicketbookManagerState {
            buffered_ticket_types,
            storage: storage.into(),
            client,
            ecash_state: Arc::new(EcashState::new(
                RequiredDepositCache::default(),
                quorum_state,
            )),
        };
        state
    }

    pub async fn attempt_assign_ticket_materials(
        &self,
        testrun_id: i32,
    ) -> anyhow::Result<AttachedTicketMaterials> {
        let mut attached_tickets = Vec::with_capacity(self.buffered_ticket_types.len());

        // make sure all epochs and expirations are covered in case we retrieved tickets from
        // different periods
        let mut coin_indices_signatures = HashMap::new();
        let mut expiration_date_signatures = HashMap::new();
        let mut master_verification_keys = HashMap::new();

        for typ in &self.buffered_ticket_types {
            debug!("attempting to get materials for ticket of type {typ}");
            if let Some(ticket) = self.storage.next_ticket(*typ, testrun_id).await? {
                let epoch_id = ticket.ticketbook.epoch_id();
                let expiration_date = ticket.ticketbook.expiration_date();

                debug!("retrieved ticket corresponds to epoch {epoch_id} and expiration date {expiration_date}");

                debug!("attempting to attach master verification key...");
                if !master_verification_keys.contains_key(&epoch_id) {
                    master_verification_keys.insert(
                        epoch_id,
                        self.master_verification_key(Some(epoch_id)).await?.clone(),
                    );
                }

                debug!("attempting to attach coin index signatures...");
                if !coin_indices_signatures.contains_key(&epoch_id) {
                    coin_indices_signatures.insert(
                        epoch_id,
                        self.master_coin_index_signatures(Some(epoch_id))
                            .await?
                            .clone(),
                    );
                }

                debug!("attempting to attach expiration date signatures...");
                if !expiration_date_signatures.contains_key(&(epoch_id, expiration_date)) {
                    expiration_date_signatures.insert(
                        (epoch_id, expiration_date),
                        self.master_expiration_date_signatures(epoch_id, expiration_date)
                            .await?
                            .clone(),
                    );
                }

                attached_tickets.push(ticket.into())
            } else {
                warn!("no tickets of type {typ} available in storage")
            }
        }

        Ok(AttachedTicketMaterials {
            coin_indices_signatures: coin_indices_signatures
                .into_values()
                .map(|s| s.pack())
                .collect(),
            expiration_date_signatures: expiration_date_signatures
                .into_values()
                .map(|s| s.pack())
                .collect(),
            master_verification_keys: master_verification_keys
                .into_iter()
                .map(|(epoch_id, key)| EpochVerificationKey { epoch_id, key }.pack())
                .collect(),
            attached_tickets,
        })
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
