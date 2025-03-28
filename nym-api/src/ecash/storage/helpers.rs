// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::NymApiStorageError;
use bincode::Options;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct StorageBorrowedSerdeWrapper<'a, T>(&'a T);

#[derive(Serialize, Deserialize)]
struct StorageSerdeWrapper<T>(T);

// SAFETY: we're not using custom serialiser for AnnotatedCoinIndexSignature
// and we're within bound limits
#[allow(clippy::unwrap_used)]
pub(crate) fn serialise_coin_index_signatures(sigs: &[AnnotatedCoinIndexSignature]) -> Vec<u8> {
    storage_serialiser()
        .serialize(&StorageBorrowedSerdeWrapper(&sigs))
        .unwrap()
}

pub(crate) fn deserialise_coin_index_signatures(
    raw: &[u8],
) -> Result<Vec<AnnotatedCoinIndexSignature>, NymApiStorageError> {
    let de: StorageSerdeWrapper<_> = storage_serialiser().deserialize(raw).map_err(|_| {
        NymApiStorageError::database_inconsistency("malformed stored coin index signatures")
    })?;
    Ok(de.0)
}

// SAFETY: we're not using custom serialiser for AnnotatedExpirationDateSignature
// and we're within bound limits
#[allow(clippy::unwrap_used)]
pub(crate) fn serialise_expiration_date_signatures(
    sigs: &[AnnotatedExpirationDateSignature],
) -> Vec<u8> {
    storage_serialiser()
        .serialize(&StorageBorrowedSerdeWrapper(&sigs))
        .unwrap()
}

pub(crate) fn deserialise_expiration_date_signatures(
    raw: &[u8],
) -> Result<Vec<AnnotatedExpirationDateSignature>, NymApiStorageError> {
    let de: StorageSerdeWrapper<_> = storage_serialiser().deserialize(raw).map_err(|_| {
        NymApiStorageError::database_inconsistency("malformed expiration date signatures")
    })?;
    Ok(de.0)
}

pub(crate) fn storage_serialiser() -> impl bincode::Options {
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
