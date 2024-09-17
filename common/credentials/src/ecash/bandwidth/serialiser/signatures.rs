// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::bandwidth::serialiser::VersionedSerialise;
use crate::Error;
use nym_credentials_interface::{AnnotatedCoinIndexSignature, AnnotatedExpirationDateSignature};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use time::Date;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AggregatedExpirationDateSignatures {
    pub epoch_id: u64,

    #[serde(with = "nym_serde_helpers::date")]
    pub expiration_date: Date,

    pub signatures: Vec<AnnotatedExpirationDateSignature>,
}

impl VersionedSerialise for AggregatedExpirationDateSignatures {
    // we start with revision 2 as the initial, 1, only contained the inner `signatures` field data
    const CURRENT_SERIALISATION_REVISION: u8 = 2;

    fn try_unpack(b: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error>
    where
        Self: DeserializeOwned,
    {
        let revision = revision
            .into()
            .unwrap_or(<Self as VersionedSerialise>::CURRENT_SERIALISATION_REVISION);

        match revision {
            1 => Err(Error::UnsupportedSerializationRevision { revision }),
            2 => Self::try_unpack_current(b),
            _ => Err(Error::UnknownSerializationRevision { revision }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AggregatedCoinIndicesSignatures {
    pub epoch_id: u64,

    pub signatures: Vec<AnnotatedCoinIndexSignature>,
}

impl VersionedSerialise for AggregatedCoinIndicesSignatures {
    // we start with revision 2 as the initial, 1, only contained the inner `signatures` field data
    const CURRENT_SERIALISATION_REVISION: u8 = 2;

    fn try_unpack(b: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error>
    where
        Self: DeserializeOwned,
    {
        let revision = revision
            .into()
            .unwrap_or(<Self as VersionedSerialise>::CURRENT_SERIALISATION_REVISION);

        match revision {
            1 => Err(Error::UnsupportedSerializationRevision { revision }),
            2 => Self::try_unpack_current(b),
            _ => Err(Error::UnknownSerializationRevision { revision }),
        }
    }
}
