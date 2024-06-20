// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NymIdError;
use nym_credential_storage::models::StorableIssuedCredential;
use nym_credential_storage::storage::Storage;
use nym_credentials::coconut::bandwidth::issued::BandwidthCredentialIssuedDataVariant;
use nym_credentials::IssuedBandwidthCredential;
use time::OffsetDateTime;
use tracing::{debug, warn};
use zeroize::Zeroizing;

pub async fn import_credential<S>(
    credentials_store: S,
    raw_credential: Vec<u8>,
    credential_version: impl Into<Option<u8>>,
) -> Result<Option<OffsetDateTime>, NymIdError>
where
    S: Storage,
    <S as Storage>::StorageError: Send + Sync + 'static,
{
    let raw_credential = Zeroizing::new(raw_credential);

    // note: the type itself implements ZeroizeOnDrop
    let credential = IssuedBandwidthCredential::try_unpack(&raw_credential, credential_version)
        .map_err(|source| NymIdError::CredentialDeserializationFailure { source })?;

    debug!(
        "attempting to import credential of type {}",
        credential.typ()
    );

    let expiry_date = match credential.variant_data() {
        BandwidthCredentialIssuedDataVariant::Voucher(voucher_info) => {
            debug!("with value of {}", voucher_info.value());
            None
        }
        BandwidthCredentialIssuedDataVariant::FreePass(freepass_info) => {
            debug!("with expiry at {}", freepass_info.expiry_date());
            if freepass_info.expired() {
                warn!("the free pass has already expired!");

                // technically we can import it, but the gateway will just reject it so what's the point
                return Err(NymIdError::ExpiredCredentialImport {
                    expiration: freepass_info.expiry_date(),
                });
            } else {
                Some(freepass_info.expiry_date())
            }
        }
    };

    // SAFETY:
    // for the epoch to run over u32::MAX, we'd have to advance it for few centuries every block...
    // the alternative is a very particularly malformed serialized data, but at that point blowing up is the right call
    // because we can't rely on it anyway
    #[allow(clippy::expect_used)]
    let storable = StorableIssuedCredential {
        serialization_revision: credential.current_serialization_revision(),
        credential_data: &raw_credential,
        credential_type: credential.typ().to_string(),
        epoch_id: credential
            .epoch_id()
            .try_into()
            .expect("our epoch is has run over u32::MAX!"),
    };

    credentials_store
        .insert_issued_credential(storable)
        .await
        .map_err(|source| NymIdError::StorageError {
            source: Box::new(source),
        })?;
    Ok(expiry_date)
}
