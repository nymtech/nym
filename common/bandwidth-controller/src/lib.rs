// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use crate::error::BandwidthControllerError;
use crate::utils::{
    get_aggregate_verification_key, get_coin_index_signatures, get_expiration_date_signatures,
    ApiClientsWrapper,
};
use log::error;
use nym_credential_storage::models::RetrievedTicketbook;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::CredentialSpendingData;
use nym_credentials_interface::{
    AnnotatedCoinIndexSignature, AnnotatedExpirationDateSignature, TicketType, VerificationKeyAuth,
};
use nym_ecash_time::Date;
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;

pub use event::BandwidthStatusMessage;

pub mod acquire;
pub mod error;
mod event;
mod utils;

#[derive(Debug)]
pub struct BandwidthController<C, St> {
    storage: St,
    client: C,
}

pub struct PreparedCredential {
    /// The cryptographic material required for spending the underlying credential.
    pub data: CredentialSpendingData,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier
    /// could use correct verification key for validation.
    pub epoch_id: EpochId,

    /// Auxiliary metadata associated with the withdrawn credential
    pub metadata: PreparedCredentialMetadata,
}

#[derive(Copy, Clone)]
pub struct PreparedCredentialMetadata {
    /// The database id of the stored credential.
    pub ticketbook_id: i64,

    /// The number of tickets withdrawn in this credential
    pub tickets_withdrawn: u32,

    /// The amount of tickets used INCLUDING those tickets that JUST got withdrawn
    pub used_tickets: u32,
}

impl<C, St: Storage> BandwidthController<C, St> {
    pub fn new(storage: St, client: C) -> Self {
        BandwidthController { storage, client }
    }

    /// Tries to retrieve one of the stored, unused credentials for the given type that hasn't yet expired.
    pub async fn get_next_usable_ticketbook(
        &self,
        ticketbook_type: TicketType,
        tickets: u32,
    ) -> Result<RetrievedTicketbook, BandwidthControllerError>
    where
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let Some(ticketbook) = self
            .storage
            .get_next_unspent_usable_ticketbook(ticketbook_type.to_string(), tickets)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
        else {
            return Err(BandwidthControllerError::NoCredentialsAvailable);
        };

        Ok(ticketbook)
    }

    pub async fn attempt_revert_ticket_usage(
        &self,
        info: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError>
    where
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        self.storage
            .attempt_revert_ticketbook_withdrawal(
                info.ticketbook_id,
                info.used_tickets,
                info.tickets_withdrawn,
            )
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    async fn get_aggregate_verification_key(
        &self,
        epoch_id: EpochId,
        apis: &mut ApiClientsWrapper,
    ) -> Result<VerificationKeyAuth, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let ecash_apis = apis.get_or_init(epoch_id, &self.client).await?;
        get_aggregate_verification_key(&self.storage, epoch_id, ecash_apis).await
    }

    async fn get_coin_index_signatures(
        &self,
        epoch_id: EpochId,
        apis: &mut ApiClientsWrapper,
    ) -> Result<Vec<AnnotatedCoinIndexSignature>, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let ecash_apis = apis.get_or_init(epoch_id, &self.client).await?;
        get_coin_index_signatures(&self.storage, epoch_id, ecash_apis).await
    }

    async fn get_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
        apis: &mut ApiClientsWrapper,
    ) -> Result<Vec<AnnotatedExpirationDateSignature>, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let ecash_apis = apis.get_or_init(epoch_id, &self.client).await?;
        get_expiration_date_signatures(&self.storage, epoch_id, expiration_date, ecash_apis).await
    }

    async fn prepare_ecash_ticket_inner(
        &self,
        provider_pk: [u8; 32],
        tickets_to_spend: u32,
        mut retrieved_ticketbook: RetrievedTicketbook,
    ) -> Result<CredentialSpendingData, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let epoch_id = retrieved_ticketbook.ticketbook.epoch_id();
        let expiration_date = retrieved_ticketbook.ticketbook.expiration_date();
        let mut api_clients = Default::default();

        let verification_key = self
            .get_aggregate_verification_key(epoch_id, &mut api_clients)
            .await?;
        let expiration_signatures = self
            .get_expiration_date_signatures(epoch_id, expiration_date, &mut api_clients)
            .await?;
        let coin_indices_signatures = self
            .get_coin_index_signatures(epoch_id, &mut api_clients)
            .await?;

        let pay_info = retrieved_ticketbook
            .ticketbook
            .generate_pay_info(provider_pk);

        let spend_request = retrieved_ticketbook.ticketbook.prepare_for_spending(
            &verification_key,
            pay_info.into(),
            &coin_indices_signatures,
            &expiration_signatures,
            tickets_to_spend as u64,
        )?;
        Ok(spend_request)
    }

    pub async fn prepare_ecash_ticket(
        &self,
        ticketbook_type: TicketType,
        provider_pk: [u8; 32],
        tickets_to_spend: u32,
    ) -> Result<PreparedCredential, BandwidthControllerError>
    where
        C: DkgQueryClient + Sync + Send,
        <St as Storage>::StorageError: Send + Sync + 'static,
    {
        let retrieved_ticketbook = self
            .get_next_usable_ticketbook(ticketbook_type, tickets_to_spend)
            .await?;

        let ticketbook_id = retrieved_ticketbook.ticketbook_id;
        let epoch_id = retrieved_ticketbook.ticketbook.epoch_id();

        let used_tickets =
            retrieved_ticketbook.ticketbook.spent_tickets() as u32 + tickets_to_spend;
        let metadata = PreparedCredentialMetadata {
            ticketbook_id,
            tickets_withdrawn: tickets_to_spend,
            used_tickets,
        };

        match self
            .prepare_ecash_ticket_inner(provider_pk, tickets_to_spend, retrieved_ticketbook)
            .await
        {
            Ok(data) => Ok(PreparedCredential {
                data,
                epoch_id,
                metadata,
            }),
            Err(err) => {
                error!("failed to prepare credential spending request. attempting to revert withdrawal...");
                self.attempt_revert_ticket_usage(metadata).await?;
                Err(err)
            }
        }
    }
}

impl<C, St> Clone for BandwidthController<C, St>
where
    C: Clone,
    St: Clone,
{
    fn clone(&self) -> Self {
        BandwidthController {
            storage: self.storage.clone(),
            client: self.client.clone(),
        }
    }
}
