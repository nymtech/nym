// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::bandwidth::CredentialSpendingData;
use crate::coconut::utils::today;
use crate::error::Error;
use nym_credentials_interface::{
    date_scalar, CoinIndexSignature, ExpirationDateSignature, PayInfo, SecretKeyUser,
    VerificationKeyAuth, Wallet,
};
use nym_network_defaults::SPEND_TICKETS;
use nym_validator_client::nym_api::EpochId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const CURRENT_SERIALIZATION_REVISION: u8 = 1;

// the only important thing to zeroize here are the private attributes, the rest can be made fully public for what we're concerned
#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct IssuedTicketBook {
    /// the underlying wallet
    wallet: Wallet,

    /// Specifies the (DKG) epoch id when this credential has been issued
    epoch_id: EpochId,

    ///secret ecash key used to generate this wallet
    ecash_secret_key: SecretKeyUser,

    ///signatures on expiration dates used to spend tickets
    #[zeroize(skip)]
    exp_date_signatures: Vec<ExpirationDateSignature>,

    ///expiration_date for easier discarding
    #[zeroize(skip)]
    expiration_date: OffsetDateTime,
}

impl IssuedTicketBook {
    pub fn new(
        wallet: Wallet,
        epoch_id: EpochId,
        ecash_secret_key: SecretKeyUser,
        exp_date_signatures: Vec<ExpirationDateSignature>,
        expiration_date: OffsetDateTime,
    ) -> Self {
        IssuedTicketBook {
            wallet,
            epoch_id,
            ecash_secret_key,
            exp_date_signatures,
            expiration_date,
        }
    }

    pub fn try_unpack(bytes: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error> {
        let revision = revision.into().unwrap_or(CURRENT_SERIALIZATION_REVISION);

        match revision {
            1 => Self::unpack_v1(bytes),
            _ => Err(Error::UnknownSerializationRevision { revision }),
        }
    }

    pub fn epoch_id(&self) -> EpochId {
        self.epoch_id
    }

    pub fn current_serialization_revision(&self) -> u8 {
        CURRENT_SERIALIZATION_REVISION
    }

    pub fn expiration_date(&self) -> OffsetDateTime {
        self.expiration_date
    }

    pub fn expired(&self) -> bool {
        self.expiration_date < today()
    }

    pub fn exp_date_sigs(&self) -> &Vec<ExpirationDateSignature> {
        &self.exp_date_signatures
    }

    pub fn wallet(&self) -> &Wallet {
        &self.wallet
    }

    /// Pack (serialize) this credential data into a stream of bytes using v1 serializer.
    pub fn pack_v1(&self) -> Vec<u8> {
        use bincode::Options;
        // safety: our data format is stable and thus the serialization should not fail
        make_storable_bincode_serializer().serialize(self).unwrap()
    }

    /// Unpack (deserialize) the credential data from the given bytes using v1 serializer.
    pub fn unpack_v1(bytes: &[u8]) -> Result<Self, Error> {
        use bincode::Options;
        make_storable_bincode_serializer()
            .deserialize(bytes)
            .map_err(|source| Error::SerializationFailure {
                source,
                revision: 1,
            })
    }

    pub fn prepare_for_spending(
        &mut self,
        verification_key: &VerificationKeyAuth,
        pay_info: PayInfo,
        coin_indices_signatures: &[CoinIndexSignature],
    ) -> Result<CredentialSpendingData, Error> {
        let params = nym_credentials_interface::ecash_parameters();
        let spend_date = today();
        let expiration_date_signatures = self.exp_date_sigs().clone();
        let payment = self.wallet.spend(
            params,
            verification_key,
            &self.ecash_secret_key,
            &pay_info,
            SPEND_TICKETS,
            &expiration_date_signatures,
            coin_indices_signatures,
            date_scalar(spend_date.unix_timestamp() as u64),
        )?;

        Ok(CredentialSpendingData {
            payment,
            pay_info,
            spend_date,
            epoch_id: self.epoch_id,
        })
    }
}

fn make_storable_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    fn assert_zeroize<T: Zeroize>() {}

    #[test]
    fn credential_is_zeroized() {
        assert_zeroize::<IssuedTicketBook>();
        assert_zeroize_on_drop::<IssuedTicketBook>();
    }
}
