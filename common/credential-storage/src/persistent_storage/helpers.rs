// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::StorageError;
use bincode::Options;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct StorageBorrowedSerdeWrapper<'a, T>(&'a T);

#[derive(Serialize, Deserialize)]
struct StorageSerdeWrapper<T>(T);

pub(crate) fn serialise_coin_index_signatures(sigs: &[AnnotatedCoinIndexSignature]) -> Vec<u8> {
    storage_serialiser()
        .serialize(&StorageBorrowedSerdeWrapper(&sigs))
        .unwrap()
}

pub(crate) fn deserialise_coin_index_signatures(
    raw: &[u8],
) -> Result<Vec<AnnotatedCoinIndexSignature>, StorageError> {
    let de: StorageSerdeWrapper<_> = storage_serialiser().deserialize(raw).map_err(|_| {
        StorageError::database_inconsistency("malformed stored coin index signatures")
    })?;
    Ok(de.0)
}

pub(crate) fn serialise_expiration_date_signatures(
    sigs: &[AnnotatedExpirationDateSignature],
) -> Vec<u8> {
    storage_serialiser()
        .serialize(&StorageBorrowedSerdeWrapper(&sigs))
        .unwrap()
}

pub(crate) fn deserialise_expiration_date_signatures(
    raw: &[u8],
) -> Result<Vec<AnnotatedExpirationDateSignature>, StorageError> {
    let de: StorageSerdeWrapper<_> = storage_serialiser().deserialize(raw).map_err(|_| {
        StorageError::database_inconsistency("malformed expiration date signatures")
    })?;
    Ok(de.0)
}

// storage serialiser used for non-critical data, such as global expiration signatures or master verification keys,
// i.e. data that could always be queried for again if malformed
fn storage_serialiser() -> impl bincode::Options {
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
