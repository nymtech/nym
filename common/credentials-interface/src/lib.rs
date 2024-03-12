// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

pub use nym_compact_ecash::{
    aggregate_verification_keys, aggregate_wallets, constants, error::CompactEcashError,
    generate_keypair_user, issue_verify,
    scheme::expiration_date_signatures::aggregate_expiration_signatures,
    scheme::expiration_date_signatures::date_scalar,
    scheme::expiration_date_signatures::ExpirationDateSignature,
    scheme::expiration_date_signatures::PartialExpirationDateSignature,
    scheme::keygen::KeyPairUser, scheme::setup::aggregate_indices_signatures,
    scheme::setup::CoinIndexSignature, scheme::setup::PartialCoinIndexSignature,
    scheme::withdrawal::RequestInfo, scheme::Payment, scheme::Wallet, setup::setup,
    setup::Parameters, utils::BlindedSignature, withdrawal_request, Base58, Bytable,
    GroupParameters, PartialWallet, PayInfo, PublicKeyUser, SecretKeyUser, VerificationKeyAuth,
    WithdrawalRequest,
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

    pub expiration_date: u64,

    pub typ: CredentialType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CredentialSpendingData {
    pub payment: Payment,

    pub pay_info: PayInfo,

    pub value: u64,

    pub typ: CredentialType,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier could use correct verification key for validation.
    pub epoch_id: u64,
}

impl CredentialSpendingData {
    pub fn verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        spend_date: u64,
    ) -> Result<bool, CompactEcashError> {
        self.payment.spend_verify(
            params,
            verification_key,
            &self.pay_info,
            date_scalar(spend_date),
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
        bytes.extend_from_slice(&self.value.to_be_bytes());
        bytes.extend_from_slice(&(typ_bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(typ_bytes);
        bytes.extend_from_slice(&self.epoch_id.to_be_bytes());

        bytes
    }

    pub fn try_from_bytes(raw: &[u8]) -> Result<Self, CompactEcashError> {
        if raw.len() < 72 + 8 + 8 + 4 + 4 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for EcashCredential deserialization".to_string(),
            ));
        }
        let mut index = 0;
        //SAFETY : casting a slice of lenght 4 into an array of size 4
        let payment_len = u32::from_be_bytes(raw[index..index + 4].try_into().unwrap()) as usize;
        index += 4;

        if raw[index..].len() < payment_len {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for EcashCredential deserialization".to_string(),
            ));
        }
        let payment = Payment::try_from(&raw[index..index + payment_len])?;
        index += payment_len;

        if raw[index..].len() < 72 + 8 + 8 + 4 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for EcashCredential deserialization".to_string(),
            ));
        }

        let pay_info = PayInfo {
            //SAFETY : casting a slice of lenght 72 into an array of size 72
            pay_info_bytes: raw[index..index + 72].try_into().unwrap(),
        };
        index += 72;

        //SAFETY : casting a slice of lenght 8 into an array of size 8
        let value = u64::from_be_bytes(raw[index..index + 8].try_into().unwrap());
        index += 8;

        //SAFETY : casting a slice of lenght 4 into an array of size 4
        let typ_len = u32::from_be_bytes(raw[index..index + 4].try_into().unwrap()) as usize;
        index += 4;

        if raw[index..].len() != typ_len + 8 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for EcashCredential deserialization".to_string(),
            ));
        }

        let raw_typ = String::from_utf8(raw[index..index + typ_len].to_vec()).map_err(|_| {
            CompactEcashError::Deserialization("Failed to deserialize type".to_string())
        })?;
        let typ = raw_typ.parse().map_err(|_| {
            CompactEcashError::Deserialization("Failed to deserialize type".to_string())
        })?;
        index += typ_len;

        //SAFETY : casting a slice of lenght 8 into an array of size 8
        let epoch_id = u64::from_be_bytes(raw[index..index + 8].try_into().unwrap());

        Ok(CredentialSpendingData {
            payment,
            pay_info,
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

pub use nym_coconut::{
    hash_to_scalar, keygen as coconut_keygen, prove_bandwidth_credential, verify_credential,
    Attribute, Base58 as CoconutBase58, BlindedSerialNumber, CoconutError,
    Parameters as CoconutParameters, Signature as CoconutSignature, VerificationKey,
    VerifyCredentialRequest,
};

//SW NOTE: for coconut compatibility
pub fn to_coconut(verification_key: &VerificationKeyAuth) -> Result<VerificationKey, CoconutError> {
    VerificationKey::from_bytes(&verification_key.to_bytes())
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct OldCredentialSpendingData {
    pub embedded_private_attributes: usize,

    pub verify_credential_request: VerifyCredentialRequest,

    pub public_attributes_plain: Vec<String>,

    pub typ: CredentialType,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier could use correct verification key for validation.
    pub epoch_id: u64,
}

impl OldCredentialSpendingData {
    pub fn verify(&self, params: &CoconutParameters, verification_key: &VerificationKey) -> bool {
        let hashed_public_attributes = self
            .public_attributes_plain
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<_>>();

        // get references to the attributes
        let public_attributes = hashed_public_attributes.iter().collect::<Vec<_>>();

        verify_credential(
            params,
            verification_key,
            &self.verify_credential_request,
            &public_attributes,
        )
    }

    pub fn validate_type_attribute(&self) -> bool {
        // the first attribute is variant specific bandwidth encoding, the second one should be the type
        let Some(type_plain) = self.public_attributes_plain.get(1) else {
            return false;
        };

        self.typ.validate(type_plain)
    }

    pub fn get_bandwidth_attribute(&self) -> Option<&String> {
        // the first attribute is variant specific bandwidth encoding, the second one should be the type
        self.public_attributes_plain.first()
    }

    pub fn blinded_serial_number(&self) -> BlindedSerialNumber {
        self.verify_credential_request.blinded_serial_number()
    }
}
