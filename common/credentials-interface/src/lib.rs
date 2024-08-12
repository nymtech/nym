// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_network_defaults::TicketTypeRepr;
use rand::Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Date, OffsetDateTime};

pub use nym_compact_ecash::{
    aggregate_verification_keys, aggregate_wallets, constants, ecash_parameters,
    error::CompactEcashError,
    generate_keypair_user, generate_keypair_user_from_seed, issue_verify,
    scheme::coin_indices_signatures::aggregate_indices_signatures,
    scheme::coin_indices_signatures::{
        AnnotatedCoinIndexSignature, CoinIndexSignature, CoinIndexSignatureShare,
        PartialCoinIndexSignature,
    },
    scheme::expiration_date_signatures::aggregate_expiration_signatures,
    scheme::expiration_date_signatures::{
        AnnotatedExpirationDateSignature, ExpirationDateSignature, ExpirationDateSignatureShare,
        PartialExpirationDateSignature,
    },
    scheme::keygen::KeyPairUser,
    scheme::withdrawal::RequestInfo,
    scheme::Payment,
    scheme::{Wallet, WalletSignatures},
    withdrawal_request, Base58, BlindedSignature, Bytable, EncodedDate, EncodedTicketType,
    PartialWallet, PayInfo, PublicKeyUser, SecretKeyUser, VerificationKeyAuth, WithdrawalRequest,
};
use nym_ecash_time::{ecash_today, EcashTime};

#[cfg(feature = "wasm-serde-types")]
use tsify::Tsify;

#[cfg(feature = "wasm-serde-types")]
use wasm_bindgen::{prelude::wasm_bindgen};

#[derive(Debug, Clone)]
pub struct CredentialSigningData {
    pub withdrawal_request: WithdrawalRequest,

    pub request_info: RequestInfo,

    pub ecash_pub_key: PublicKeyUser,

    pub expiration_date: Date,

    pub ticketbook_type: TicketType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CredentialSpendingData {
    pub payment: Payment,

    pub pay_info: PayInfo,

    pub spend_date: Date,

    // pub value: u64,
    /// The (DKG) epoch id under which the credential has been issued so that the verifier could use correct verification key for validation.
    pub epoch_id: u64,
}

impl CredentialSpendingData {
    pub fn verify(&self, verification_key: &VerificationKeyAuth) -> Result<(), CompactEcashError> {
        self.payment.spend_verify(
            verification_key,
            &self.pay_info,
            self.spend_date.ecash_unix_timestamp(),
        )
    }

    pub fn encoded_serial_number(&self) -> Vec<u8> {
        self.payment.encoded_serial_number()
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
        bytes.extend_from_slice(&self.spend_date.to_julian_day().to_be_bytes());
        bytes.extend_from_slice(&self.epoch_id.to_be_bytes());

        bytes
    }

    pub fn try_from_bytes(raw: &[u8]) -> Result<Self, CompactEcashError> {
        // minimum length: 72 (pay_info) + 8 (epoch_id) + 4 (spend date) + 4 (payment length prefix)
        if raw.len() < 72 + 8 + 4 + 4 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }
        let mut index = 0;
        //SAFETY : casting a slice of length 4 into an array of size 4
        let payment_len = u32::from_be_bytes(raw[index..index + 4].try_into().unwrap()) as usize;
        index += 4;

        if raw[index..].len() != payment_len + 84 {
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

        //SAFETY : casting a slice of length 4 into an array of size 4
        let spend_date_julian = i32::from_be_bytes(raw[index..index + 4].try_into().unwrap());
        let spend_date = Date::from_julian_day(spend_date_julian).map_err(|_| {
            CompactEcashError::DeserializationFailure {
                object: "CredentialSpendingData".into(),
            }
        })?;
        index += 4;

        if raw[index..].len() != 8 {
            return Err(CompactEcashError::DeserializationFailure {
                object: "EcashCredential".into(),
            });
        }

        //SAFETY : casting a slice of length 8 into an array of size 8
        let epoch_id = u64::from_be_bytes(raw[index..].try_into().unwrap());

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

#[derive(
    Default,
    Copy,
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(feature = "wasm-serde-types", derive(Tsify))]
#[cfg_attr(feature = "wasm-serde-types", tsify(into_wasm_abi, from_wasm_abi))]
pub enum TicketType {
    #[default]
    V1MixnetEntry,
    V1MixnetExit,
    V1WireguardEntry,
    V1WireguardExit,
}

#[derive(Debug, Copy, Clone, Error)]
#[error("provided unknown ticketbook type")]
pub struct UnknownTicketType;

impl TicketType {
    pub fn to_repr(&self) -> TicketTypeRepr {
        (*self).into()
    }

    pub fn encode(&self) -> EncodedTicketType {
        self.to_repr() as EncodedTicketType
    }

    pub fn try_from_encoded(val: EncodedTicketType) -> Result<Self, UnknownTicketType> {
        match val {
            n if n == TicketTypeRepr::V1MixnetEntry as u8 => {
                Ok(TicketTypeRepr::V1MixnetEntry.into())
            }
            n if n == TicketTypeRepr::V1MixnetExit as u8 => Ok(TicketTypeRepr::V1MixnetExit.into()),
            n if n == TicketTypeRepr::V1WireguardEntry as u8 => {
                Ok(TicketTypeRepr::V1WireguardEntry.into())
            }
            n if n == TicketTypeRepr::V1WireguardExit as u8 => {
                Ok(TicketTypeRepr::V1WireguardExit.into())
            }
            _ => Err(UnknownTicketType),
        }
    }
}

impl From<TicketType> for TicketTypeRepr {
    fn from(value: TicketType) -> Self {
        match value {
            TicketType::V1MixnetEntry => TicketTypeRepr::V1MixnetEntry,
            TicketType::V1MixnetExit => TicketTypeRepr::V1MixnetExit,
            TicketType::V1WireguardEntry => TicketTypeRepr::V1WireguardEntry,
            TicketType::V1WireguardExit => TicketTypeRepr::V1WireguardExit,
        }
    }
}

impl From<TicketTypeRepr> for TicketType {
    fn from(value: TicketTypeRepr) -> Self {
        match value {
            TicketTypeRepr::V1MixnetEntry => TicketType::V1MixnetEntry,
            TicketTypeRepr::V1MixnetExit => TicketType::V1MixnetExit,
            TicketTypeRepr::V1WireguardEntry => TicketType::V1WireguardEntry,
            TicketTypeRepr::V1WireguardExit => TicketType::V1WireguardExit,
        }
    }
}

#[derive(Clone)]
pub struct ClientTicket {
    pub spending_data: CredentialSpendingData,
    pub ticket_id: i64,
}

impl ClientTicket {
    pub fn new(spending_data: CredentialSpendingData, ticket_id: i64) -> Self {
        ClientTicket {
            spending_data,
            ticket_id,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AvailableBandwidth {
    pub bytes: i64,
    pub expiration: OffsetDateTime,
}

impl AvailableBandwidth {
    pub fn expired(&self) -> bool {
        self.expiration < ecash_today()
    }
}

impl Default for AvailableBandwidth {
    fn default() -> Self {
        Self {
            bytes: 0,
            expiration: OffsetDateTime::UNIX_EPOCH,
        }
    }
}
