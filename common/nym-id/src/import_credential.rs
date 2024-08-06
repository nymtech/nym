// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymIdError;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_credentials::ecash::utils::EcashTime;
use nym_credentials::IssuedTicketBook;
use time::OffsetDateTime;
use tracing::{debug, warn};
use zeroize::Zeroizing;

pub async fn import_credential<S>(
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
        .map_err(|source| NymIdError::CredentialDeserializationFailure { source })?;

    debug!(
        "attempting to import credential with expiration date at {}",
        ticketbook.expiration_date()
    );

    if ticketbook.expired() {
        warn!("the credential has already expired!");

        // technically we can import it, but the gateway will just reject it so what's the point
        return Err(NymIdError::ExpiredCredentialImport {
            expiration: ticketbook.expiration_date(),
        });
    }

    credentials_store
        .insert_issued_ticketbook(&ticketbook)
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?;
    Ok(ticketbook.expiration_date().ecash_datetime())
}
