// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub use nym_coconut::{
    aggregate_signature_shares, aggregate_verification_keys, blind_sign, hash_to_scalar,
    prepare_blind_sign, prove_bandwidth_credential, verify_credential, Attribute, Base58,
    BlindSignRequest, BlindedSignature, Bytable, CoconutError, KeyPair, Parameters,
    PrivateAttribute, PublicAttribute, SecretKey, Signature, SignatureShare, VerificationKey,
    VerifyCredentialRequest,
};

pub const VOUCHER_INFO_TYPE: &str = "BandwidthVoucher";
pub const FREE_PASS_INFO_TYPE: &str = "FreeBandwidthPass";

// pub trait NymCredential {
//     fn prove_credential(&self) -> Result<(), ()>;
// }

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CredentialType {
    Voucher,
    FreePass,
}

impl CredentialType {
    pub fn validate(&self, type_plain: &str) -> bool {
        match self {
            CredentialType::Voucher => type_plain == VOUCHER_INFO_TYPE,
            CredentialType::FreePass => type_plain == FREE_PASS_INFO_TYPE,
        }
    }

    pub fn is_free_pass(&self) -> bool {
        matches!(self, CredentialType::FreePass)
    }

    pub fn is_voucher(&self) -> bool {
        matches!(self, CredentialType::Voucher)
    }
}

impl Display for CredentialType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialType::Voucher => VOUCHER_INFO_TYPE.fmt(f),
            CredentialType::FreePass => FREE_PASS_INFO_TYPE.fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CredentialSigningData {
    pub pedersen_commitments_openings: Vec<Scalar>,

    pub blind_sign_request: BlindSignRequest,

    pub public_attributes_plain: Vec<String>,

    pub typ: CredentialType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialSpendingData {
    pub embedded_private_attributes: usize,

    pub verify_credential_request: VerifyCredentialRequest,

    pub public_attributes_plain: Vec<String>,

    pub typ: CredentialType,
}

impl CredentialSpendingData {
    pub fn verify(&self, params: &Parameters, verification_key: &VerificationKey) -> bool {
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
}
