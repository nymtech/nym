// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use crate::requests::{BandwidthControllerRequest, BandwidthControllerRequestSender};
use crate::traits::CredentialPublicDataFetcher;
use crate::{
    BandwidthTicketProvider, PreparedCredential, PreparedCredentialMetadata, UPGRADE_MODE_JWT_TYPE,
};
use async_trait::async_trait;
use log::error;
use nym_credential_storage::models::RetrievedTicketbook;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::CredentialSpendingData;
use nym_credentials_interface::{
    AnnotatedCoinIndexSignature, AnnotatedExpirationDateSignature, TicketType, VerificationKeyAuth,
};
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::{Date, OffsetDateTime};
use nym_task::ShutdownToken;
use nym_validator_client::nym_api::EpochId;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub struct BandwidthController<St> {
    storage: St,

    // Channels used to receive commands from the outside.
    request_channel: (
        UnboundedSender<BandwidthControllerRequest>,
        UnboundedReceiver<BandwidthControllerRequest>,
    ),

    // fetches the global ecash signing materials when they are missing locally
    public_data_fetcher: Option<Box<dyn CredentialPublicDataFetcher>>,
}

impl<St: Storage> BandwidthController<St> {
    pub fn new(storage: St) -> Self {
        let request_channel = mpsc::unbounded_channel();
        BandwidthController {
            storage,
            request_channel,
            public_data_fetcher: None,
        }
    }

    #[must_use]
    pub fn with_credential_public_data_fetcher(
        mut self,
        fetcher: impl CredentialPublicDataFetcher + 'static,
    ) -> Self {
        self.public_data_fetcher = Some(Box::new(fetcher));
        self
    }

    /// Get the request channel used to send request to the controller.
    /// Request are handled only if the `BandwidthController` is running using `run`
    pub fn get_request_sender(&self) -> BandwidthControllerRequestSender {
        BandwidthControllerRequestSender::new(self.request_channel.0.clone())
    }

    /// Tries to retrieve one of the stored, unused credentials for the given type that hasn't yet expired.
    pub async fn get_next_usable_ticketbook(
        &self,
        ticketbook_type: TicketType,
        tickets: u32,
    ) -> Result<Option<RetrievedTicketbook>, BandwidthControllerError> {
        self.storage
            .get_next_unspent_usable_ticketbook(ticketbook_type.to_string(), tickets)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    pub async fn attempt_revert_ticket_usage(
        &self,
        info: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        self.storage
            .attempt_revert_ticketbook_withdrawal(
                info.ticketbook_id,
                info.tickets_withdrawn,
                info.used_tickets,
            )
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    pub async fn get_aggregate_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<VerificationKeyAuth>, BandwidthControllerError> {
        self.storage
            .get_master_verification_key(epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    pub async fn get_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Option<Vec<AnnotatedCoinIndexSignature>>, BandwidthControllerError> {
        self.storage
            .get_coin_index_signatures(epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    pub async fn get_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<Option<Vec<AnnotatedExpirationDateSignature>>, BandwidthControllerError> {
        self.storage
            .get_expiration_date_signatures(expiration_date, epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    /// Returns the master verification key for the epoch, fetching it via the configured public
    /// data fetcher and persisting it if it isn't already in local storage.
    async fn ensure_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<VerificationKeyAuth, BandwidthControllerError> {
        if let Some(key) = self.get_aggregate_verification_key(epoch_id).await? {
            return Ok(key);
        }
        let Some(fetcher) = &self.public_data_fetcher else {
            return Err(BandwidthControllerError::MissingVerificationKey { epoch_id });
        };
        let key = fetcher
            .fetch_master_verification_key(epoch_id)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.storage
            .insert_master_verification_key(&key)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        Ok(key.key)
    }

    /// Returns the coin index signatures for the epoch, fetching them via the configured public
    /// data fetcher and persisting them if they aren't already in local storage.
    async fn ensure_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<AnnotatedCoinIndexSignature>, BandwidthControllerError> {
        if let Some(signatures) = self.get_coin_index_signatures(epoch_id).await? {
            return Ok(signatures);
        }
        let Some(fetcher) = &self.public_data_fetcher else {
            return Err(BandwidthControllerError::MissingCoinIndexSignatures { epoch_id });
        };
        let signatures = fetcher
            .fetch_coin_index_signatures(epoch_id)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.storage
            .insert_coin_index_signatures(&signatures)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        Ok(signatures.signatures)
    }

    /// Returns the expiration date signatures for the epoch and expiration date, fetching them via
    /// the configured public data fetcher and persisting them if they aren't already in local storage.
    async fn ensure_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<Vec<AnnotatedExpirationDateSignature>, BandwidthControllerError> {
        if let Some(signatures) = self
            .get_expiration_date_signatures(epoch_id, expiration_date)
            .await?
        {
            return Ok(signatures);
        }
        let Some(fetcher) = &self.public_data_fetcher else {
            return Err(BandwidthControllerError::MissingExpirationDateSignatures { epoch_id });
        };
        let signatures = fetcher
            .fetch_expiration_date_signatures(expiration_date, epoch_id)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.storage
            .insert_expiration_date_signatures(&signatures)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        Ok(signatures.signatures)
    }

    /// Ensures all global ecash signing materials for the given epoch and expiration date are
    /// present in local storage, fetching and persisting any that are missing.
    pub async fn ensure_global_data(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<(), BandwidthControllerError> {
        self.ensure_master_verification_key(epoch_id).await?;
        self.ensure_coin_index_signatures(epoch_id).await?;
        self.ensure_expiration_date_signatures(epoch_id, expiration_date)
            .await?;
        Ok(())
    }

    async fn prepare_ecash_ticket_inner(
        &self,
        provider_pk: [u8; 32],
        spend_time: OffsetDateTime,
        tickets_to_spend: u32,
        mut retrieved_ticketbook: RetrievedTicketbook,
    ) -> Result<CredentialSpendingData, BandwidthControllerError> {
        let epoch_id = retrieved_ticketbook.ticketbook.epoch_id();
        let expiration_date = retrieved_ticketbook.ticketbook.expiration_date();

        let verification_key = self.ensure_master_verification_key(epoch_id).await?;
        let expiration_signatures = self
            .ensure_expiration_date_signatures(epoch_id, expiration_date)
            .await?;
        let coin_indices_signatures = self.ensure_coin_index_signatures(epoch_id).await?;

        let pay_info = retrieved_ticketbook
            .ticketbook
            .generate_pay_info(provider_pk, spend_time);

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
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        let Some(retrieved_ticketbook) = self
            .get_next_usable_ticketbook(ticketbook_type, tickets_to_spend)
            .await?
        else {
            return Ok(None);
        };

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
            .prepare_ecash_ticket_inner(
                provider_pk,
                spend_time,
                tickets_to_spend,
                retrieved_ticketbook,
            )
            .await
        {
            Ok(data) => Ok(Some(PreparedCredential {
                data,
                epoch_id,
                metadata,
            })),
            Err(err) => {
                error!("failed to prepare credential spending request. attempting to revert withdrawal...");
                self.attempt_revert_ticket_usage(metadata).await?;
                Err(err)
            }
        }
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        let Some(emergency_credential) = self
            .storage
            .get_emergency_credential(UPGRADE_MODE_JWT_TYPE)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
        else {
            return Ok(None);
        };
        // upgrade mode credential is just a simple stringified JWT
        let token = String::from_utf8(emergency_credential.data.content)
            .map_err(|_| BandwidthControllerError::MalformedUpgradeModeToken)?;
        Ok(Some(token))
    }

    /// Runs the controller event loop, handling incoming requests until the
    /// request channel is closed or cancellation is requested.
    pub async fn run(mut self, shutdown_token: ShutdownToken) {
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    log::debug!("bandwidth controller received cancellation request; shutting down");
                    break;
                }
                request = self.request_channel.1.recv() => match request {
                    Some(request) => self.handle_request(request).await,
                    None => {
                        log::warn!("bandwidth controller request channel closed; this should never happened as we are owning a sender; shutting down");
                        break;
                    }
                }
            }
        }

        self.storage.close().await;
    }

    async fn handle_request(&mut self, request: BandwidthControllerRequest) {
        match request {
            BandwidthControllerRequest::EcashTicket(return_sender, request) => {
                let credential_result = self
                    .prepare_ecash_ticket(
                        request.ticket_type,
                        request.gateway_id.to_bytes(),
                        request.tickets_to_spend,
                        request.spend_time,
                    )
                    .await;
                return_sender.send(credential_result)
            }
            BandwidthControllerRequest::UpgradeModeToken(return_sender) => {
                return_sender.send(self.get_upgrade_mode_token().await)
            }
            BandwidthControllerRequest::AttemptRevertSpending(return_sender, metadata) => {
                return_sender.send(self.attempt_revert_ticket_usage(metadata).await)
            }
        }
    }
}

// So we can use the BC without making it run on its own if we don't need that
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<St: Storage> BandwidthTicketProvider for BandwidthController<St> {
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        self.prepare_ecash_ticket(
            ticket_type,
            gateway_id.to_bytes(),
            tickets_to_spend,
            spend_time,
        )
        .await
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        self.get_upgrade_mode_token().await
    }

    async fn attempt_revert_spending(
        &self,
        metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        self.attempt_revert_ticket_usage(metadata).await
    }

    async fn close(&self) {
        self.storage.close().await
    }
}
