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
    BandwidthTicketProvider, EcashTicketRequest, PreparedCredential, PreparedCredentialMetadata,
};

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
            .map_err(BandwidthControllerError::internal)?;
        rx.await.map_err(BandwidthControllerError::internal)?
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        let (tx, rx) = ReturnSender::new();
        self.command_tx
            .send(BandwidthControllerRequest::UpgradeModeToken(tx))
            .map_err(BandwidthControllerError::internal)?;
        rx.await.map_err(BandwidthControllerError::internal)?
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
            .map_err(BandwidthControllerError::internal)?;
        rx.await.map_err(BandwidthControllerError::internal)?
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
