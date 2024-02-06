// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use nym_coconut_interface::{BlindSignRequest, Parameters, VerifyCredentialRequest};
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
pub fn bandwidth_voucher_params() -> &'static Parameters {
    static BANDWIDTH_CREDENTIAL_PARAMS: OnceLock<Parameters> = OnceLock::new();
    BANDWIDTH_CREDENTIAL_PARAMS.get_or_init(IssuanceBandwidthCredential::default_parameters)
}

#[derive(Zeroize, ZeroizeOnDrop, Clone, Debug)]
pub enum CredentialType {
    Voucher,
    FreePass,
}

impl CredentialType {
    pub fn is_free_pass(&self) -> bool {
        matches!(self, CredentialType::FreePass)
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
    pub(crate) pedersen_commitments_openings: Vec<Scalar>,

    pub(crate) blind_sign_request: BlindSignRequest,

    pub(crate) public_attributes_plain: Vec<String>,
}

#[derive(Debug)]
pub struct CredentialSpendingData {
    pub(crate) verify_credential_request: VerifyCredentialRequest,

    pub(crate) public_attributes_plain: Vec<String>,
}
