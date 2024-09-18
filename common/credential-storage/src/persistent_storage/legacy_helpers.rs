// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::StorageError;
use bincode::Options;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::VerificationKeyAuth;
use serde::Deserialize;

#[derive(Deserialize)]
struct StorageSerdeWrapper<T>(T);

pub(crate) fn deserialise_v1_expiration_date_signatures(
    raw: &[u8],
) -> Result<Vec<AnnotatedExpirationDateSignature>, StorageError> {
    let de: StorageSerdeWrapper<_> = v1_signatures_serialiser().deserialize(raw).map_err(|_| {
        StorageError::database_inconsistency("malformed expiration date signatures")
    })?;
    Ok(de.0)
}

pub(crate) fn deserialise_v1_coin_index_signatures(
    raw: &[u8],
) -> Result<Vec<AnnotatedCoinIndexSignature>, StorageError> {
    let de: StorageSerdeWrapper<_> = v1_signatures_serialiser().deserialize(raw).map_err(|_| {
        StorageError::database_inconsistency("malformed stored coin index signatures")
    })?;
    Ok(de.0)
}

pub(crate) fn deserialise_v1_master_verification_key(
    raw: &[u8],
) -> Result<VerificationKeyAuth, StorageError> {
    VerificationKeyAuth::from_bytes(raw).map_err(|_| {
        StorageError::database_inconsistency("malformed stored master verification key")
    })
}

fn v1_signatures_serialiser() -> impl bincode::Options {
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
