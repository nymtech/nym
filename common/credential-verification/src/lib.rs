// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::traits::EcashManager;
use async_trait::async_trait;
use bandwidth_storage_manager::BandwidthStorageManager;
use nym_credentials::ecash::utils::{EcashTime, cred_exp_date, ecash_today};
use nym_credentials_interface::{Bandwidth, ClientTicket, TicketType};
use nym_gateway_requests::models::CredentialSpendingRequest;
use std::sync::Arc;
use std::time::Instant;
use time::{Date, OffsetDateTime};
use tracing::*;

pub use client_bandwidth::*;
pub use error::*;

pub mod bandwidth_storage_manager;
mod client_bandwidth;
pub mod ecash;
pub mod error;

// Histogram buckets for ecash verification duration (in seconds)
const ECASH_VERIFICATION_DURATION_BUCKETS: &[f64] =
    &[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0];

pub struct CredentialVerifier {
    credential: CredentialSpendingRequest,
    ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
    bandwidth_storage_manager: BandwidthStorageManager,
}

impl CredentialVerifier {
    pub fn new(
        credential: CredentialSpendingRequest,
        ecash_verifier: Arc<dyn EcashManager + Send + Sync>,
        bandwidth_storage_manager: BandwidthStorageManager,
    ) -> Self {
        CredentialVerifier {
            credential,
            ecash_verifier,
            bandwidth_storage_manager,
        }
    }

    fn check_credential_spending_date(&self, today: Date) -> Result<()> {
        let proposed = self.credential.data.spend_date;
        trace!("checking ticket spending date...");

        if today != proposed {
            trace!("invalid credential spending date. received {proposed}");
            return Err(Error::InvalidCredentialSpendingDate {
                got: proposed,
                expected: today,
            });
        }
        Ok(())
    }

    async fn check_local_db_for_double_spending(&self, serial_number: &[u8]) -> Result<()> {
        trace!("checking local db for double spending...");

        let spent = self
            .bandwidth_storage_manager
            .storage
            .contains_ticket(serial_number)
            .await?;
        if spent {
            trace!("the credential has already been spent before at this gateway");
            nym_metrics::inc!("ecash_verification_failures_double_spending");
            return Err(Error::BandwidthCredentialAlreadySpent);
        }
        Ok(())
    }

    async fn cryptographically_verify_ticket(&self) -> Result<()> {
        trace!("attempting to perform ticket verification...");

        let aggregated_verification_key = self
            .ecash_verifier
            .verification_key(self.credential.data.epoch_id)
            .await?;

        self.ecash_verifier
            .check_payment(&self.credential.data, &aggregated_verification_key)
            .await?;
        Ok(())
    }

    async fn store_received_ticket(&self, received_at: OffsetDateTime) -> Result<i64> {
        trace!("storing received ticket");
        let ticket_id = self
            .bandwidth_storage_manager
            .storage
            .insert_received_ticket(
                self.bandwidth_storage_manager.client_id,
                received_at,
                self.credential.encoded_serial_number(),
                self.credential.to_bytes(),
            )
            .await?;
        Ok(ticket_id)
    }

    fn async_verify_ticket(&self, ticket_id: i64) {
        let client_ticket = ClientTicket::new(self.credential.data.clone(), ticket_id);

        self.ecash_verifier.async_verify(client_ticket);
    }

    pub async fn verify(&mut self) -> Result<i64> {
        let start = Instant::now();
        nym_metrics::inc!("ecash_verification_attempts");

        let received_at = OffsetDateTime::now_utc();
        let spend_date = ecash_today();

        // check if the credential hasn't been spent before
        let serial_number = self.credential.data.encoded_serial_number();
        let credential_type = TicketType::try_from_encoded(self.credential.data.payment.t_type)?;

        if self.credential.data.payment.spend_value != 1 {
            nym_metrics::inc!("ecash_verification_failures_multiple_tickets");
            return Err(Error::MultipleTickets);
        }

        if let Err(e) = self.check_credential_spending_date(spend_date.ecash_date()) {
            nym_metrics::inc!("ecash_verification_failures_invalid_spend_date");
            return Err(e);
        }

        self.check_local_db_for_double_spending(&serial_number)
            .await?;

        // TODO: do we HAVE TO do it?
        let verify_result = self.cryptographically_verify_ticket().await;

        // Track verification duration
        let duration = start.elapsed().as_secs_f64();
        nym_metrics::add_histogram_obs!(
            "ecash_verification_duration_seconds",
            duration,
            ECASH_VERIFICATION_DURATION_BUCKETS
        );

        // Track epoch ID - use dynamic metric name via registry
        let epoch_id = self.credential.data.epoch_id;
        let epoch_metric = format!(
            "nym_credential_verification_ecash_epoch_{}_verifications",
            epoch_id
        );
        nym_metrics::metrics_registry().maybe_register_and_inc(&epoch_metric, None);

        // Check verification result after timing
        verify_result?;

        let ticket_id = self.store_received_ticket(received_at).await?;
        self.async_verify_ticket(ticket_id);

        // TODO: double storing?
        // self.store_spent_credential(serial_number_bs58).await?;

        let bandwidth = Bandwidth::ticket_amount(credential_type.into());

        self.bandwidth_storage_manager
            .increase_bandwidth(bandwidth, cred_exp_date())
            .await?;

        nym_metrics::inc!("ecash_verification_success");

        Ok(self
            .bandwidth_storage_manager
            .client_bandwidth
            .available()
            .await)
    }
}

#[async_trait]
pub trait TicketVerifier {
    /// Verify that the ticket is valid and cryptographically correct.
    /// If the verification succeeds, also increase the bandwidth with the ticket's
    /// amount and return the latest available bandwidth
    async fn verify(&mut self) -> Result<i64>;
}

#[async_trait]
impl TicketVerifier for CredentialVerifier {
    async fn verify(&mut self) -> Result<i64> {
        self.verify().await
    }
}
