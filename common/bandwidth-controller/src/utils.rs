// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::BandwidthControllerError;
use nym_credential_storage::models::StoredIssuedCredential;
use nym_credentials::coconut::bandwidth::issued::CURRENT_SERIALIZATION_REVISION;
use nym_credentials::coconut::bandwidth::IssuedTicketBook;

pub fn stored_credential_to_issued_bandwidth(
    cred: StoredIssuedCredential,
) -> Result<IssuedTicketBook, BandwidthControllerError> {
    if cred.serialization_revision != CURRENT_SERIALIZATION_REVISION {
        return Err(
            BandwidthControllerError::UnsupportedCredentialStorageRevision {
                stored: cred.serialization_revision,
                expected: CURRENT_SERIALIZATION_REVISION,
            },
        );
    }

    Ok(IssuedTicketBook::unpack_v1(&cred.credential_data)?)
}
