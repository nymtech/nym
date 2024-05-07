// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::OnceLock;

pub use issuance::IssuanceBandwidthCredential;
pub use issued::IssuedBandwidthCredential;
pub use nym_credentials_interface::{
    CredentialSigningData, CredentialSpendingData, CredentialType, Parameters,
    UnknownCredentialType,
};

pub mod freepass;
pub mod issuance;
pub mod issued;
pub mod voucher;

pub fn bandwidth_credential_params() -> &'static Parameters {
    static BANDWIDTH_CREDENTIAL_PARAMS: OnceLock<Parameters> = OnceLock::new();
    BANDWIDTH_CREDENTIAL_PARAMS.get_or_init(IssuanceBandwidthCredential::default_parameters)
}
