// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use rand::Rng;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone)]
pub struct CredentialSigningData {
    pub withdrawal_request: WithdrawalRequest,

    pub request_info: RequestInfo,

    pub ecash_pub_key: PublicKeyUser,

    pub expiration_date: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CredentialSpendingData {
    pub payment: Payment,

    pub pay_info: PayInfo,

    pub spend_date: OffsetDateTime,

    // pub value: u64,
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

        bytes.extend_from_slice(&(payment_bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payment_bytes);
        bytes.extend_from_slice(&self.pay_info.pay_info_bytes); //this is 72 bytes long
        bytes.extend_from_slice(&self.spend_date.unix_timestamp().to_be_bytes());
        bytes.extend_from_slice(&self.epoch_id.to_be_bytes());

        bytes
    }

    pub fn try_from_bytes(raw: &[u8]) -> Result<Self, CompactEcashError> {
        // minimum length: 72 (pay_info) + 8 (epoch_id) + 8 (spend date TS) + 4 (payment length prefix)
        if raw.len() < 72 + 8 + 8 + 4 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }
        let mut index = 0;
        //SAFETY : casting a slice of length 4 into an array of size 4
        let payment_len = u32::from_be_bytes(raw[index..index + 4].try_into().unwrap()) as usize;
        index += 4;

        if raw[index..].len() != payment_len + 88 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }
        let payment = Payment::try_from(&raw[index..index + payment_len])?;
        index += payment_len;

        let pay_info = PayInfo {
            //SAFETY : casting a slice of length 72 into an array of size 72
            pay_info_bytes: raw[index..index + 72].try_into().unwrap(),
        };
        index += 72;

        //SAFETY : casting a slice of length 8 into an array of size 8
        let spend_date_timestamp = i64::from_be_bytes(raw[index..index + 8].try_into().unwrap());
        let spend_date =
            OffsetDateTime::from_unix_timestamp(spend_date_timestamp).map_err(|_| {
                CompactEcashError::DeserializationFailure {
                    object: "CredentialSpendingData".into(),
                }
            })?;
        index += 8;

        if raw[index..].len() != 8 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }

        //SAFETY : casting a slice of length 8 into an array of size 8
        let epoch_id = u64::from_be_bytes(raw[index..index + 8].try_into().unwrap());

        Ok(CredentialSpendingData {
            payment,
            pay_info,
            spend_date,
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
