// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet::CredentialStorage;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    IssuedTicketBook,
};

/// Helper for importing bandwidth credentials (ticketbooks) into a mixnet client.
///
/// `BandwidthImporter` provides methods for importing the various cryptographic
/// components needed for paid network access before connecting to the mixnet.
///
/// ## Overview
///
/// To use the Nym mixnet with paid bandwidth, clients need:
/// 1. **Ticketbooks**: Pre-purchased bandwidth tokens that are spent during network use
/// 2. **Verification keys**: Cryptographic keys to verify credential signatures
/// 3. **Signatures**: Aggregated signatures for coin indices and expiration dates
///
/// ## Usage
///
/// Obtain a `BandwidthImporter` by calling
/// [`DisconnectedMixnetClient::begin_bandwidth_import`](crate::mixnet::DisconnectedMixnetClient::begin_bandwidth_import)
/// on a disconnected client:
///
/// ```rust,no_run
/// use nym_sdk::mixnet::MixnetClientBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = MixnetClientBuilder::new_ephemeral()
///     .build()?;
///
/// // Import credentials before connecting
/// {
///     let importer = client.begin_bandwidth_import();
///     // importer.import_ticketbook(&ticketbook).await?;
///     // importer.import_master_verification_key(&key).await?;
///     // ...
/// } // importer dropped here, releasing the borrow on client
///
/// // Now connect with credentials available
/// let connected = client.connect_to_mixnet().await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Credential Components
///
/// - **Ticketbook**: Contains bandwidth tokens that are spent during network use
/// - **Master verification key**: Used to verify credential signatures for an epoch
/// - **Coin index signatures**: Signatures over the coin indices in the ticketbook
/// - **Expiration date signatures**: Signatures over credential expiration dates
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

    /// Imports a complete ticketbook into credential storage.
    ///
    /// A ticketbook contains pre-purchased bandwidth tokens that are spent
    /// during network use. Each token represents a certain amount of bandwidth.
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

    /// Imports a partial range of tickets from a ticketbook.
    ///
    /// Useful when sharing a ticketbook across multiple clients or when only
    /// a portion should be available to this client.
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

    /// Imports the master verification key for credential validation.
    ///
    /// Used to verify that credentials were properly issued by the credential
    /// signers. Each epoch has its own verification key.
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

    /// Imports aggregated coin index signatures.
    ///
    /// These signatures are needed to prove ownership of specific
    /// coins/tokens in the ticketbook.
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

    /// Imports aggregated expiration date signatures.
    ///
    /// These signatures verify the validity period of credentials. Credentials
    /// are only valid for a certain time period, and these signatures prove
    /// when they expire.
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
