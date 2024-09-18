// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymIdError;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
use nym_credentials::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::ecash::utils::EcashTime;
use nym_credentials::{ImportableTicketBook, IssuedTicketBook};
use time::OffsetDateTime;
use zeroize::Zeroizing;

mod helpers;

pub async fn import_master_verification_key<S>(
    credentials_store: S,
    raw_key: Vec<u8>,
    key_version: impl Into<Option<u8>>,
) -> Result<(), NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    let key = EpochVerificationKey::try_unpack(&raw_key, key_version)
        .map_err(|source| NymIdError::VerificationKeyDeserializationFailure { source })?;

    helpers::import_master_verification_key(&credentials_store, &key).await
}

pub async fn import_expiration_date_signatures<S>(
    credentials_store: S,
    raw_signatures: Vec<u8>,
    signatures_version: impl Into<Option<u8>>,
) -> Result<(), NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    let signatures =
        AggregatedExpirationDateSignatures::try_unpack(&raw_signatures, signatures_version)
            .map_err(
                |source| NymIdError::ExpirationDateSignaturesDeserializationFailure { source },
            )?;

    helpers::import_expiration_date_signatures(&credentials_store, &signatures).await
}

pub async fn import_coin_index_signatures<S>(
    credentials_store: S,
    raw_signatures: Vec<u8>,
    signatures_version: impl Into<Option<u8>>,
) -> Result<(), NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    let signatures =
        AggregatedCoinIndicesSignatures::try_unpack(&raw_signatures, signatures_version)
            .map_err(|source| NymIdError::CoinIndexSignaturesDeserializationFailure { source })?;

    helpers::import_coin_index_signatures(&credentials_store, &signatures).await
}

pub async fn import_standalone_ticketbook<S>(
    credentials_store: S,
    raw_credential: Vec<u8>,
    credential_version: impl Into<Option<u8>>,
) -> Result<OffsetDateTime, NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    let raw_credential = Zeroizing::new(raw_credential);

    // note: the type itself implements ZeroizeOnDrop
    let ticketbook = IssuedTicketBook::try_unpack(&raw_credential, credential_version)
        .map_err(|source| NymIdError::TicketbookDeserializationFailure { source })?;

    helpers::import_ticketbook(&credentials_store, &ticketbook).await?;
    Ok(ticketbook.expiration_date().ecash_datetime())
}

pub async fn import_full_ticketbook<S>(
    credentials_store: S,
    raw_credential: Vec<u8>,
    credential_version: impl Into<Option<u8>>,
) -> Result<OffsetDateTime, NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    let raw_credential = Zeroizing::new(raw_credential);

    let importable = ImportableTicketBook::try_unpack(&raw_credential, credential_version)
        .map_err(|source| NymIdError::FullTicketbookDeserializationFailure { source })?;

    let decoded = importable
        .try_unpack_full()
        .map_err(|source| NymIdError::TicketbookDeserializationFailure { source })?;

    if let Some(key) = &decoded.master_verification_key {
        helpers::import_master_verification_key(&credentials_store, key).await?
    }

    if let Some(sigs) = &decoded.expiration_date_signatures {
        helpers::import_expiration_date_signatures(&credentials_store, sigs).await?
    }

    if let Some(sigs) = &decoded.coin_index_signatures {
        helpers::import_coin_index_signatures(&credentials_store, sigs).await?
    }

    helpers::import_ticketbook(&credentials_store, &decoded.ticketbook).await?;

    Ok(decoded.ticketbook.expiration_date().ecash_datetime())
}
