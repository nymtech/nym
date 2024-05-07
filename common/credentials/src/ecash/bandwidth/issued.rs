// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::bandwidth::serialiser::VersionedSerialise;
use crate::ecash::bandwidth::CredentialSpendingData;
use crate::ecash::utils::ecash_today;
use crate::error::Error;
use nym_credentials_interface::{
    CoinIndexSignature, ExpirationDateSignature, PayInfo, SecretKeyUser, VerificationKeyAuth,
    Wallet, WalletSignatures,
};
use nym_ecash_time::EcashTime;
use nym_validator_client::nym_api::EpochId;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use time::Date;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const CURRENT_SERIALIZATION_REVISION: u8 = 1;

// the only important thing to zeroize here are the private attributes, the rest can be made fully public for what we're concerned
#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct IssuedTicketBook {
    /// the underlying wallet signatures
    signatures_wallet: WalletSignatures,

    /// the counter indicating how many tickets have been spent so far
    spent_tickets: u64,

    /// Specifies the (DKG) epoch id when this credential has been issued
    epoch_id: EpochId,

    /// secret ecash key used to generate this wallet
    ecash_secret_key: SecretKeyUser,

    /// expiration_date for easier discarding
    #[zeroize(skip)]
    expiration_date: Date,
}

impl IssuedTicketBook {
    pub fn new(
        wallet: WalletSignatures,
        epoch_id: EpochId,
        ecash_secret_key: SecretKeyUser,
        expiration_date: Date,
    ) -> Self {
        IssuedTicketBook {
            signatures_wallet: wallet,
            spent_tickets: 0,
            epoch_id,
            ecash_secret_key,
            expiration_date,
        }
    }

    pub fn from_parts(
        signatures_wallet: WalletSignatures,
        epoch_id: EpochId,
        ecash_secret_key: SecretKeyUser,
        expiration_date: Date,
        spent_tickets: u64,
    ) -> Self {
        IssuedTicketBook {
            signatures_wallet,
            spent_tickets,
            epoch_id,
            ecash_secret_key,
            expiration_date,
        }
    }

    pub fn update_spent_tickets(&mut self, spent_tickets: u64) {
        self.spent_tickets = spent_tickets
    }

    pub fn epoch_id(&self) -> EpochId {
        self.epoch_id
    }

    pub fn current_serialization_revision(&self) -> u8 {
        CURRENT_SERIALIZATION_REVISION
    }

    pub fn expiration_date(&self) -> Date {
        self.expiration_date
    }

    pub fn expired(&self) -> bool {
        self.expiration_date < ecash_today().date()
    }

    pub fn params_total_tickets(&self) -> u64 {
        nym_credentials_interface::ecash_parameters().get_total_coins()
    }

    pub fn spent_tickets(&self) -> u64 {
        self.spent_tickets
    }

    pub fn wallet(&self) -> &WalletSignatures {
        &self.signatures_wallet
    }

    pub fn prepare_for_spending<BI, BE>(
        &mut self,
        verification_key: &VerificationKeyAuth,
        pay_info: PayInfo,
        coin_indices_signatures: &[BI],
        expiration_date_signatures: &[BE],
        tickets_to_spend: u64,
    ) -> Result<CredentialSpendingData, Error>
    where
        BI: Borrow<CoinIndexSignature>,
        BE: Borrow<ExpirationDateSignature>,
    {
        let params = nym_credentials_interface::ecash_parameters();
        let spend_date = ecash_today();

        // make sure we still have enough tickets to spend
        Wallet::ensure_allowance(params, self.spent_tickets, tickets_to_spend)?;

        let payment = self.signatures_wallet.spend(
            params,
            verification_key,
            &self.ecash_secret_key,
            &pay_info,
            self.spent_tickets,
            tickets_to_spend,
            expiration_date_signatures,
            coin_indices_signatures,
            spend_date.ecash_unix_timestamp(),
        )?;

        self.spent_tickets += tickets_to_spend;

        Ok(CredentialSpendingData {
            payment,
            pay_info,
            spend_date: spend_date.ecash_date(),
            epoch_id: self.epoch_id,
        })
    }
}

impl VersionedSerialise for IssuedTicketBook {
    const CURRENT_SERIALISATION_REVISION: u8 = 1;

    fn try_unpack(b: &[u8], revision: impl Into<Option<u8>>) -> Result<Self, Error> {
        let revision = revision
            .into()
            .unwrap_or(<Self as VersionedSerialise>::CURRENT_SERIALISATION_REVISION);

        match revision {
            1 => Self::try_unpack_current(b),
            _ => Err(Error::UnknownSerializationRevision { revision }),
        }
    }
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
