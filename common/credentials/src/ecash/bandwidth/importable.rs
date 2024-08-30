// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
use crate::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use crate::ecash::bandwidth::{
    issued::IssuedTicketBook,
    serialiser::{VersionSerialised, VersionedSerialise},
};
use crate::Error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub struct DecodedImportableTicketBook {
    pub ticketbook: IssuedTicketBook,

    pub expiration_date_signatures: Option<AggregatedExpirationDateSignatures>,

    pub coin_index_signatures: Option<AggregatedCoinIndicesSignatures>,

    pub master_verification_key: Option<EpochVerificationKey>,
}

#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct ImportableTicketBook {
    pub serialised_ticketbook: VersionSerialised<IssuedTicketBook>,

    #[zeroize(skip)]
    pub serialised_expiration_date_signatures:
        Option<VersionSerialised<AggregatedExpirationDateSignatures>>,

    #[zeroize(skip)]
    pub serialised_coin_index_signatures:
        Option<VersionSerialised<AggregatedCoinIndicesSignatures>>,

    #[zeroize(skip)]
    pub serialised_master_verification_key: Option<VersionSerialised<EpochVerificationKey>>,
}

impl From<IssuedTicketBook> for ImportableTicketBook {
    fn from(ticketbook: IssuedTicketBook) -> Self {
        ImportableTicketBook {
            serialised_ticketbook: ticketbook.pack(),
            serialised_expiration_date_signatures: None,
            serialised_coin_index_signatures: None,
            serialised_master_verification_key: None,
        }
    }
}

impl ImportableTicketBook {
    pub fn with_expiration_date_signatures(
        &mut self,
        signatures: &AggregatedExpirationDateSignatures,
    ) -> &mut Self {
        self.serialised_expiration_date_signatures = Some(signatures.pack());
        self
    }

    pub fn with_coin_index_signatures(
        &mut self,
        signatures: &AggregatedCoinIndicesSignatures,
    ) -> &mut Self {
        self.serialised_coin_index_signatures = Some(signatures.pack());
        self
    }

    pub fn with_master_verification_key(&mut self, key: &EpochVerificationKey) -> &mut Self {
        self.serialised_master_verification_key = Some(key.pack());
        self
    }

    pub fn finalize_export(self) -> Vec<u8> {
        self.pack().data
    }

    pub fn try_unpack_full(&self) -> Result<DecodedImportableTicketBook, Error> {
        Ok(DecodedImportableTicketBook {
            ticketbook: self.serialised_ticketbook.try_unpack()?,
            expiration_date_signatures: self
                .serialised_expiration_date_signatures
                .as_ref()
                .map(|sigs| sigs.try_unpack())
                .transpose()?,
            coin_index_signatures: self
                .serialised_coin_index_signatures
                .as_ref()
                .map(|sigs| sigs.try_unpack())
                .transpose()?,
            master_verification_key: self
                .serialised_master_verification_key
                .as_ref()
                .map(|key| key.try_unpack())
                .transpose()?,
        })
    }
}

impl VersionedSerialise for ImportableTicketBook {
    const CURRENT_SERIALISATION_REVISION: u8 = 1;

    fn try_unpack(b: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error>
    where
        Self: DeserializeOwned,
    {
        let revision = revision
            .into()
            .unwrap_or(<Self as VersionedSerialise>::CURRENT_SERIALISATION_REVISION);

        match revision {
            1 => Self::try_unpack_current(b),
            _ => Err(Error::UnknownSerializationRevision { revision }),
        }
    }
}
