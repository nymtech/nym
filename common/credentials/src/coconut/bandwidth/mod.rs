// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use nym_coconut_interface::{
    hash_to_scalar, BlindSignRequest, Parameters, VerificationKey, VerifyCredentialRequest,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub use issuance::IssuanceBandwidthCredential;
pub use issued::IssuedBandwidthCredential;

pub mod freepass;
pub mod issuance;
pub mod issued;
pub mod voucher;

pub const VOUCHER_INFO_TYPE: &str = "BandwidthVoucher";
pub const FREE_PASS_INFO_TYPE: &str = "FreeBandwidthPass";

// works under the assumption of having 4 attributes in the underlying credential(s)
pub fn bandwidth_credential_params() -> &'static Parameters {
    static BANDWIDTH_CREDENTIAL_PARAMS: OnceLock<Parameters> = OnceLock::new();
    BANDWIDTH_CREDENTIAL_PARAMS.get_or_init(IssuanceBandwidthCredential::default_parameters)
}

#[derive(Zeroize, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum CredentialType {
    Voucher,
    FreePass,
}

impl CredentialType {
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
    pub fn verify(&self, verification_key: &VerificationKey) -> bool {
        let params = bandwidth_credential_params();

        let hashed_public_attributes = self
            .public_attributes_plain
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<_>>();

        // get references to the attributes
        let public_attributes = hashed_public_attributes.iter().collect::<Vec<_>>();

        nym_coconut_interface::verify_credential(
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

        match self.typ {
            CredentialType::Voucher => type_plain == VOUCHER_INFO_TYPE,
            CredentialType::FreePass => type_plain == FREE_PASS_INFO_TYPE,
        }
    }

    pub fn get_bandwidth_attribute(&self) -> Option<&String> {
        // the first attribute is variant specific bandwidth encoding, the second one should be the type
        self.public_attributes_plain.first()
    }
}
