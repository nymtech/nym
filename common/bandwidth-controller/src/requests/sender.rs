// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::OffsetDateTime;
use tokio::sync::mpsc::UnboundedSender;
use tracing::instrument;

use crate::{
    error::BandwidthControllerError,
    requests::{BandwidthControllerRequest, ReturnSender},
    ticketbooks::AvailableTicketbooks,
    traits::CredentialFetcher,
    BandwidthTicketProvider, CredentialPublicDataFetcher, EcashTicketRequest, PreparedCredential,
    PreparedCredentialMetadata,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct BandwidthControllerRequestSender {
    command_tx: UnboundedSender<BandwidthControllerRequest>,
}

// Basic set of commands that can be sent to the bandwidth controller

impl BandwidthControllerRequestSender {
    pub fn new(command_tx: UnboundedSender<BandwidthControllerRequest>) -> Self {
        Self { command_tx }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::EcashTicket(
                tx,
                EcashTicketRequest {
                    ticket_type,
                    gateway_id,
                    tickets_to_spend,
                    spend_time,
                },
            ))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::UpgradeModeToken(tx))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn attempt_revert_spending(
        &self,
        metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::AttemptRevertSpending(
                tx, metadata,
            ))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Installs the credential fetcher; the controller immediately restocks any low types.
    #[instrument(skip(self, credential_fetcher))]
    pub async fn set_credential_fetcher(
        &self,
        credential_fetcher: Arc<impl CredentialFetcher + 'static>,
    ) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::SetCredentialFetcher(
                tx,
                Some(credential_fetcher),
            ))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Removes the credential fetcher; no further automatic restocking happens until one is set.
    #[instrument(skip(self))]
    pub async fn unset_credential_fetcher(&self) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::SetCredentialFetcher(tx, None))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Installs the global-data fetcher used to retrieve missing ecash signing materials.
    #[instrument(skip(self, public_data_fetcher))]
    pub async fn set_public_data_fetcher(
        &self,
        public_data_fetcher: Arc<impl CredentialPublicDataFetcher + 'static>,
    ) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::SetPublicDataFetcher(
                tx,
                Some(public_data_fetcher),
            ))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Removes the global-data fetcher.
    #[instrument(skip(self))]
    pub async fn unset_public_data_fetcher(&self) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::SetPublicDataFetcher(tx, None))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Cancels in-flight fetches, drops the fetcher, clears stored credentials, and fails any parked
    /// readiness waiters. Used to fully de-provision the controller.
    #[instrument(skip(self))]
    pub async fn reset(&self) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::Reset(tx))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Removes stored emergency (upgrade-mode) credentials, leaving ticketbooks intact.
    #[instrument(skip(self))]
    pub async fn clear_emergency_credentials(&self) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::ClearEmergencyCredentials(tx))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Returns the currently stored ticketbooks.
    #[instrument(skip(self))]
    pub async fn get_available_ticketbooks(
        &self,
    ) -> Result<AvailableTicketbooks, BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::GetAvailableTicketbooks(tx))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }

    /// Kicks off a background restock for the given ticket types
    /// Returns once the restock is scheduled - it does not wait for the fetches to finish
    /// (use [`Self::wait_for_ticketbooks`] for that).
    ///
    /// Not to be used lightly: the automatic triggers (ticket handout, timer, fetcher install)
    /// already keep every type stocked. This is a manual safety valve, not a routine call.
    #[instrument(skip(self))]
    #[doc(hidden)]
    pub async fn restock_ticketbooks(
        &self,
        types: Vec<TicketType>,
    ) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::RestockTicketbooks(tx, types))
            .map_err(BandwidthControllerError::internal)?;
        rx.await.map_err(BandwidthControllerError::internal)?
    }

    /// Kicks off a background restock for every ticket type running low or about to expire.
    /// Returns once the restock is scheduled, not once the fetches finish.
    #[instrument(skip(self))]
    #[doc(hidden)]
    pub async fn restock_all_ticketbooks(&self) -> Result<(), BandwidthControllerError> {
        self.restock_ticketbooks(AvailableTicketbooks::ticketbook_types())
            .await
    }

    /// Resolves once every listed type is usable (stocked or covered by upgrade mode). Errors if a
    /// required type is neither stocked nor being fetched; otherwise waits while the unsatisfied
    /// ones are still in flight.
    #[instrument(skip(self))]
    pub async fn wait_for_ticketbooks(
        &self,
        types: Vec<TicketType>,
    ) -> Result<(), BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::WaitForTicketbooks(tx, types))
            .map_err(|_| BandwidthControllerError::ChannelClosed)?;
        rx.await
            .map_err(|_| BandwidthControllerError::ChannelClosed)?
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl BandwidthTicketProvider for BandwidthControllerRequestSender {
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        self.get_ecash_ticket(ticket_type, gateway_id, tickets_to_spend, spend_time)
            .await
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        self.get_upgrade_mode_token().await
    }

    async fn attempt_revert_spending(
        &self,
        metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        self.attempt_revert_spending(metadata).await
    }

    // No-op, the controller will close when stopped
    async fn close(&self) {}
}
