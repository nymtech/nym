// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymIdError;
use nym_credential_storage::storage::Storage;
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
    IssuedTicketBook,
};
use tracing::{debug, warn};

pub(crate) async fn import_master_verification_key<S>(
    credentials_store: &S,
    key: &EpochVerificationKey,
) -> Result<(), NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    debug!(
        "attempting to import master verification key for epoch {}",
        key.epoch_id
    );

    credentials_store
        .insert_master_verification_key(key)
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?;
    Ok(())
}

pub(crate) async fn import_expiration_date_signatures<S>(
    credentials_store: &S,
    signatures: &AggregatedExpirationDateSignatures,
) -> Result<(), NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    debug!(
        "attempting to import expiration date signatures with expiration date at {} (epoch: {})",
        signatures.expiration_date, signatures.epoch_id
    );

    credentials_store
        .insert_expiration_date_signatures(signatures)
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?;
    Ok(())
}

pub(crate) async fn import_coin_index_signatures<S>(
    credentials_store: &S,
    signatures: &AggregatedCoinIndicesSignatures,
) -> Result<(), NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    debug!(
        "attempting to import coin index signatures for epoch {}",
        signatures.epoch_id
    );

    credentials_store
        .insert_coin_index_signatures(signatures)
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?;
    Ok(())
}

pub(crate) async fn import_ticketbook<S>(
    credentials_store: &S,
    ticketbook: &IssuedTicketBook,
) -> Result<(), NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    debug!(
        "attempting to import ticketbook with expiration date at {}",
        ticketbook.expiration_date()
    );

    if ticketbook.expired() {
        warn!("the credential has already expired!");

        // technically we can import it, but the gateway will just reject it so what's the point
        return Err(NymIdError::ExpiredCredentialImport {
            expiration: ticketbook.expiration_date(),
        });
    }

    // in order to import the ticketbook we MUST have the appropriate signatures in the storage already
    if credentials_store
        .get_expiration_date_signatures(ticketbook.expiration_date())
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?
        .is_none()
    {
        return Err(NymIdError::MissingExpirationDateSignatures {
            date: ticketbook.expiration_date(),
        });
    }

    if credentials_store
        .get_coin_index_signatures(ticketbook.epoch_id())
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?
        .is_none()
    {
        return Err(NymIdError::MissingCoinIndexSignatures {
            epoch_id: ticketbook.epoch_id(),
        });
    }

    if credentials_store
        .get_master_verification_key(ticketbook.epoch_id())
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?
        .is_none()
    {
        return Err(NymIdError::MissingMasterVerificationKey {
            epoch_id: ticketbook.epoch_id(),
        });
    }

    credentials_store
        .insert_issued_ticketbook(ticketbook)
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?;
    Ok(())
}
