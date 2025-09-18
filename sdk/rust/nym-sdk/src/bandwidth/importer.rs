// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet::CredentialStorage;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    IssuedTicketBook,
};

/// Represents a helper that can be used for importing ticketbooks / bandwidth
/// into the client before commencing mixnet connection
/// The way to create it is to call
/// [`crate::mixnet::DisconnectedMixnetClient::begin_bandwidth_import`] on the associated mixnet
/// client.
pub struct BandwidthImporter<'a, St> {
    storage: &'a St,
}

impl<'a, St> BandwidthImporter<'a, St>
where
    St: CredentialStorage,
    <St as CredentialStorage>::StorageError: Send + Sync + 'static,
{
    pub(crate) fn new(storage: &'a St) -> Self {
        BandwidthImporter { storage }
    }

    pub async fn import_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
    ) -> Result<(), crate::Error> {
        self.storage
            .insert_issued_ticketbook(ticketbook)
            .await
            .map_err(|err| crate::Error::CredentialStorageError {
                source: Box::new(err),
            })?;
        Ok(())
    }

    pub async fn import_partial_ticketbook(
        &self,
        ticketbook: &IssuedTicketBook,
        allowed_start_ticket_index: u32,
        allowed_final_ticket_index: u32,
    ) -> Result<(), crate::Error> {
        self.storage
            .insert_partial_issued_ticketbook(
                ticketbook,
                allowed_start_ticket_index,
                allowed_final_ticket_index,
            )
            .await
            .map_err(|err| crate::Error::CredentialStorageError {
                source: Box::new(err),
            })?;
        Ok(())
    }

    pub async fn import_master_verification_key(
        &self,
        key: &EpochVerificationKey,
    ) -> Result<(), crate::Error> {
        self.storage
            .insert_master_verification_key(key)
            .await
            .map_err(|err| crate::Error::CredentialStorageError {
                source: Box::new(err),
            })?;
        Ok(())
    }

    pub async fn import_coin_index_signatures(
        &self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> Result<(), crate::Error> {
        self.storage
            .insert_coin_index_signatures(signatures)
            .await
            .map_err(|err| crate::Error::CredentialStorageError {
                source: Box::new(err),
            })?;
        Ok(())
    }

    pub async fn import_expiration_date_signatures(
        &self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> Result<(), crate::Error> {
        self.storage
            .insert_expiration_date_signatures(signatures)
            .await
            .map_err(|err| crate::Error::CredentialStorageError {
                source: Box::new(err),
            })?;
        Ok(())
    }
}
