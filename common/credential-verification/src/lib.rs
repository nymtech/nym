// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bandwidth_storage_manager::BandwidthStorageManager;
use std::sync::Arc;
use time::{Date, OffsetDateTime};
use tracing::*;

use nym_credentials::ecash::utils::{ecash_today, EcashTime};
use nym_credentials_interface::{Bandwidth, ClientTicket, TicketType};
use nym_gateway_requests::models::CredentialSpendingRequest;
use nym_gateway_storage::Storage;

pub use client_bandwidth::*;
use ecash::EcashManager;
pub use error::*;

pub mod bandwidth_storage_manager;
mod client_bandwidth;
pub mod ecash;
pub mod error;

pub struct CredentialVerifier<S> {
    credential: CredentialSpendingRequest,
    ecash_verifier: Arc<EcashManager<S>>,
    bandwidth_storage_manager: BandwidthStorageManager<S>,
}

impl<S: Storage + Clone + 'static> CredentialVerifier<S> {
    pub fn new(
        credential: CredentialSpendingRequest,
        ecash_verifier: Arc<EcashManager<S>>,
        bandwidth_storage_manager: BandwidthStorageManager<S>,
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
        let received_at = OffsetDateTime::now_utc();
        let spend_date = ecash_today();

        // check if the credential hasn't been spent before
        let serial_number = self.credential.data.encoded_serial_number();
        let credential_type = TicketType::try_from_encoded(self.credential.data.payment.t_type)?;

        if self.credential.data.payment.spend_value != 1 {
            return Err(Error::MultipleTickets);
        }

        self.check_credential_spending_date(spend_date.ecash_date())?;
        self.check_local_db_for_double_spending(&serial_number)
            .await?;

        // TODO: do we HAVE TO do it?
        self.cryptographically_verify_ticket().await?;

        let ticket_id = self.store_received_ticket(received_at).await?;
        self.async_verify_ticket(ticket_id);

        // TODO: double storing?
        // self.store_spent_credential(serial_number_bs58).await?;

        let bandwidth = Bandwidth::ticket_amount(credential_type.into());

        self.bandwidth_storage_manager
            .increase_bandwidth(bandwidth, spend_date)
            .await?;

        Ok(self
            .bandwidth_storage_manager
            .client_bandwidth
            .available()
            .await)
    }
}
