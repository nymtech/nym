// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// for time being assume the bandwidth credential consists of public identity of the requester
// and private (though known... just go along with it) infinite bandwidth value
// right now this has no double-spending protection, spender binding, etc
// it's the simplest possible case

use url::Url;

use crate::error::Error;
use crate::utils::{obtain_aggregate_signature, prepare_credential_for_spending, ValidatorInfo};
use coconut_interface::{hash_to_scalar, Credential, Parameters, Signature, VerificationKey};

const BANDWIDTH_VALUE: u64 = 10 * 1024 * 1024 * 1024; // 10 GB

pub const PUBLIC_ATTRIBUTES: u32 = 1;
pub const PRIVATE_ATTRIBUTES: u32 = 1;
pub const TOTAL_ATTRIBUTES: u32 = PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES;

// TODO: this definitely has to be moved somewhere else. It's just a temporary solution
pub async fn obtain_signature(raw_identity: &[u8], validators: &[Url], verification_key: &VerificationKey) -> Result<Signature, Error> {
    let public_attributes = vec![hash_to_scalar(BANDWIDTH_VALUE.to_be_bytes())];
    let private_attributes = vec![hash_to_scalar(raw_identity)];

    let params = Parameters::new(TOTAL_ATTRIBUTES)?;

    obtain_aggregate_signature(&params, &public_attributes, &private_attributes, validators, verification_key).await
}

pub fn prepare_for_spending(
    raw_identity: &[u8],
    signature: &Signature,
    verification_key: &VerificationKey,
) -> Result<Credential, Error> {
    let public_attributes = vec![BANDWIDTH_VALUE.to_be_bytes().to_vec()];
    let private_attributes = vec![raw_identity.to_vec()];

    let params = Parameters::new(TOTAL_ATTRIBUTES)?;

    prepare_credential_for_spending(
        &params,
        public_attributes,
        private_attributes,
        signature,
        verification_key,
    )
}
