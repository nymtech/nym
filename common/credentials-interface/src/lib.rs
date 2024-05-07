// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;
use time::OffsetDateTime;

pub use nym_compact_ecash::{
    aggregate_verification_keys, aggregate_wallets, constants, ecash_parameters,
    error::CompactEcashError,
    generate_keypair_user, generate_keypair_user_from_seed, issue_verify,
    scheme::coin_indices_signatures::aggregate_indices_signatures,
    scheme::coin_indices_signatures::{
        CoinIndexSignature, CoinIndexSignatureShare, PartialCoinIndexSignature,
    },
    scheme::expiration_date_signatures::aggregate_expiration_signatures,
    scheme::expiration_date_signatures::date_scalar,
    scheme::expiration_date_signatures::{
        ExpirationDateSignature, ExpirationDateSignatureShare, PartialExpirationDateSignature,
    },
    scheme::keygen::KeyPairUser,
    scheme::withdrawal::RequestInfo,
    scheme::Payment,
    scheme::Wallet,
    withdrawal_request, Base58, BlindedSignature, Bytable, PartialWallet, PayInfo, PublicKeyUser,
    SecretKeyUser, VerificationKeyAuth, WithdrawalRequest,
};

pub const ECASH_INFO_TYPE: &str = "TicketBook";
pub const FREE_PASS_INFO_TYPE: &str = "FreeBandwidthPass";

// pub trait NymCredential {
//     fn prove_credential(&self) -> Result<(), ()>;
// }

#[derive(Debug, Error)]
#[error("{0} is not a valid credential type")]
pub struct UnknownCredentialType(String);

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CredentialType {
    TicketBook,
    FreePass,
}

impl FromStr for CredentialType {
    type Err = UnknownCredentialType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == ECASH_INFO_TYPE {
            Ok(CredentialType::TicketBook)
        } else if s == FREE_PASS_INFO_TYPE {
            Ok(CredentialType::FreePass)
        } else {
            Err(UnknownCredentialType(s.to_string()))
        }
    }
}

impl CredentialType {
    pub fn validate(&self, type_plain: &str) -> bool {
        match self {
            CredentialType::TicketBook => type_plain == ECASH_INFO_TYPE,
            CredentialType::FreePass => type_plain == FREE_PASS_INFO_TYPE,
        }
    }

    pub fn is_free_pass(&self) -> bool {
        matches!(self, CredentialType::FreePass)
    }

    pub fn is_ticketbook(&self) -> bool {
        matches!(self, CredentialType::TicketBook)
    }
}

impl Display for CredentialType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialType::TicketBook => ECASH_INFO_TYPE.fmt(f),
            CredentialType::FreePass => FREE_PASS_INFO_TYPE.fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CredentialSigningData {
    pub withdrawal_request: WithdrawalRequest,

    pub request_info: RequestInfo,

    pub ecash_pub_key: PublicKeyUser,

    pub expiration_date: OffsetDateTime,

    pub typ: CredentialType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CredentialSpendingData {
    pub payment: Payment,

    pub pay_info: PayInfo,

    pub spend_date: OffsetDateTime,

    pub value: u64,

    pub typ: CredentialType,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier could use correct verification key for validation.
    pub epoch_id: u64,
}

impl CredentialSpendingData {
    pub fn verify(
        &self,
        verification_key: &VerificationKeyAuth,
    ) -> Result<bool, CompactEcashError> {
        self.payment.spend_verify(
            verification_key,
            &self.pay_info,
            date_scalar(self.spend_date.unix_timestamp() as u64),
        )
    }

    pub fn serial_number_b58(&self) -> String {
        self.payment.serial_number_bs58()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // simple length prefixed serialization
        // TODO: change it to a standard format instead
        let mut bytes = Vec::new();
        let payment_bytes = self.payment.to_bytes();
        let typ = self.typ.to_string();
        let typ_bytes = typ.as_bytes();

        bytes.extend_from_slice(&(payment_bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payment_bytes);
        bytes.extend_from_slice(&self.pay_info.pay_info_bytes); //this is 72 bytes long
        bytes.extend_from_slice(&self.spend_date.unix_timestamp().to_be_bytes());
        bytes.extend_from_slice(&self.value.to_be_bytes());
        bytes.extend_from_slice(&(typ_bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(typ_bytes);
        bytes.extend_from_slice(&self.epoch_id.to_be_bytes());

        bytes
    }

    pub fn try_from_bytes(raw: &[u8]) -> Result<Self, CompactEcashError> {
        if raw.len() < 72 + 8 + 8 + 8 + 4 + 4 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }
        let mut index = 0;
        //SAFETY : casting a slice of lenght 4 into an array of size 4
        let payment_len = u32::from_be_bytes(raw[index..index + 4].try_into().unwrap()) as usize;
        index += 4;

        if raw[index..].len() < payment_len {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }
        let payment = Payment::try_from(&raw[index..index + payment_len])?;
        index += payment_len;

        if raw[index..].len() < 72 + 8 + 8 + 8 + 4 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }

        let pay_info = PayInfo {
            //SAFETY : casting a slice of lenght 72 into an array of size 72
            pay_info_bytes: raw[index..index + 72].try_into().unwrap(),
        };
        index += 72;

        //SAFETY : casting a slice of lenght 8 into an array of size 8
        let spend_date_timestamp = i64::from_be_bytes(raw[index..index + 8].try_into().unwrap());
        let spend_date =
            OffsetDateTime::from_unix_timestamp(spend_date_timestamp).map_err(|_| {
                CompactEcashError::DeserializationFailure {
                    object: "CredentialSpendingData".into(),
                }
            })?;
        index += 8;

        //SAFETY : casting a slice of lenght 8 into an array of size 8
        let value = u64::from_be_bytes(raw[index..index + 8].try_into().unwrap());
        index += 8;

        //SAFETY : casting a slice of lenght 4 into an array of size 4
        let typ_len = u32::from_be_bytes(raw[index..index + 4].try_into().unwrap()) as usize;
        index += 4;

        if raw[index..].len() != typ_len + 8 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }

        let raw_typ = String::from_utf8(raw[index..index + typ_len].to_vec()).map_err(|_| {
            CompactEcashError::DeserializationFailure {
                object: "Credential type".into(),
            }
        })?;
        let typ = raw_typ
            .parse()
            .map_err(|_| CompactEcashError::DeserializationFailure {
                object: "Credential type".into(),
            })?;
        index += typ_len;

        //SAFETY : casting a slice of lenght 8 into an array of size 8
        let epoch_id = u64::from_be_bytes(raw[index..index + 8].try_into().unwrap());

        Ok(CredentialSpendingData {
            payment,
            pay_info,
            spend_date,
            value,
            typ,
            epoch_id,
        })
    }
}

impl Bytable for CredentialSpendingData {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CompactEcashError> {
        Self::try_from_bytes(slice)
    }
}

impl Base58 for CredentialSpendingData {}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct NymPayInfo {
    randomness: [u8; 32],
    timestamp: i64,
    provider_public_key: [u8; 32],
}

impl NymPayInfo {
    /// Generates a new `NymPayInfo` instance with random bytes, a timestamp, and a provider public key.
    ///
    /// # Arguments
    ///
    /// * `provider_pk` - The public key of the payment provider.
    ///
    /// # Returns
    ///
    /// A new `NymPayInfo` instance.
    ///
    pub fn generate(provider_pk: [u8; 32]) -> Self {
        let mut randomness = [0u8; 32];
        rand::thread_rng().fill(&mut randomness[..32]);

        let timestamp = OffsetDateTime::now_utc().unix_timestamp();

        NymPayInfo {
            randomness,
            timestamp,
            provider_public_key: provider_pk,
        }
    }

    pub fn timestamp(&self) -> i64 {
        self.timestamp
    }

    pub fn pk(&self) -> [u8; 32] {
        self.provider_public_key
    }
}

impl From<NymPayInfo> for PayInfo {
    fn from(value: NymPayInfo) -> Self {
        let mut pay_info_bytes = [0u8; 72];

        pay_info_bytes[..32].copy_from_slice(&value.randomness);
        pay_info_bytes[32..40].copy_from_slice(&value.timestamp.to_be_bytes());
        pay_info_bytes[40..].copy_from_slice(&value.provider_public_key);

        PayInfo { pay_info_bytes }
    }
}

impl From<PayInfo> for NymPayInfo {
    fn from(value: PayInfo) -> Self {
        //SAFETY : slice to array of same length
        let randomness = value.pay_info_bytes[..32].try_into().unwrap();
        let timestamp = i64::from_be_bytes(value.pay_info_bytes[32..40].try_into().unwrap());
        let provider_public_key = value.pay_info_bytes[40..].try_into().unwrap();

        NymPayInfo {
            randomness,
            timestamp,
            provider_public_key,
        }
    }
}
