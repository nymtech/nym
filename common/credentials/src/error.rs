// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::CompactEcashError;
use nym_crypto::asymmetric::encryption::KeyRecoveryError;
use nym_validator_client::ValidatorClientError;

use crate::coconut::bandwidth::issued::CURRENT_SERIALIZATION_REVISION;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error")]
    IOError(#[from] std::io::Error),

    #[error("failed to deserialize a recovery credential: {source}")]
    RecoveryCredentialDeserializationFailure { source: bincode::Error },

    #[error("failed to (de)serialize provided credential using revision {revision}: {source}")]
    SerializationFailure {
        #[source]
        source: bincode::Error,
        revision: u8,
    },

    #[error("unknown credential serializatio revision {revision}. the current (and max supported) version is {CURRENT_SERIALIZATION_REVISION}")]
    UnknownSerializationRevision { revision: u8 },

    #[error("The detailed description is yet to be determined")]
    BandwidthCredentialError,

    #[error("Could not contact any validator")]
    NoValidatorsAvailable,

    #[error("Ran into a Compact ecash error - {0}")]
    CompactEcashError(#[from] CompactEcashError),

    #[error("Ran into a validator client error - {0}")]
    ValidatorClientError(#[from] ValidatorClientError),

    #[error("Bandwidth operation overflowed. {0}")]
    BandwidthOverflow(String),

    #[error("There is not associated bandwidth for the given client")]
    MissingBandwidth,

    #[error("Could not parse the key - {0}")]
    ParsePublicKey(#[from] KeyRecoveryError),

    #[error("Could not gather enough signature shares. Try again using the recovery command")]
    NotEnoughShares,

    #[error("Could not aggregate signature shares - {0}. Try again using the recovery command")]
    SignatureAggregationError(CompactEcashError),

    #[error("Could not deserialize bandwidth voucher - {0}")]
    BandwidthVoucherDeserializationError(String),

    #[error("the provided issuance data wasn't prepared for a bandwidth voucher")]
    NotABandwdithVoucher,

    #[error("failed to create a secp256k1 signature")]
    Secp256k1SignFailure,
}
