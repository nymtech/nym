// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// for time being assume the bandwidth credential consists of public identity of the requester
// and private (though known... just go along with it) infinite bandwidth value
// right now this has no double-spending protection, spender binding, etc
// it's the simplest possible case

use url::Url;

use crate::error::Error;
use crate::utils::{obtain_aggregate_signature, prepare_credential_for_spending, ValidatorInfo};
use coconut_interface::{hash_to_scalar, Credential, Parameters, Signature, VerificationKey, Attribute, PrivateAttribute, PublicAttribute};

pub const BANDWIDTH_VALUE: u64 = 10 * 1024 * 1024 * 1024; // 10 GB

pub const PUBLIC_ATTRIBUTES: u32 = 2;
pub const PRIVATE_ATTRIBUTES: u32 = 2;
pub const TOTAL_ATTRIBUTES: u32 = PUBLIC_ATTRIBUTES + PRIVATE_ATTRIBUTES;

pub const SERIAL_NUMBER_LEN: usize = 47;
pub const BINDING_NUMBER_LEN: usize = 47;
pub const VOUCHER_INFO_LEN: usize = 47;


pub struct BandwidthVoucherAttributes {
    pub serial_number : PrivateAttribute,
    pub binding_number : PrivateAttribute,
    pub voucher_value : PublicAttribute,
    pub voucher_info : PublicAttribute,
}

impl BandwidthVoucherAttributes {

    pub fn get_public_attributes(&self) -> Vec<PublicAttribute> {
        let mut pub_attributes = Vec::with_capacity(PUBLIC_ATTRIBUTES as usize);
        pub_attributes.push(self.voucher_value);
        pub_attributes.push(self.voucher_info);
        pub_attributes
    }

    pub fn get_private_attributes(&self) -> Vec<PrivateAttribute> {
        let mut priv_attributes = Vec::with_capacity(PRIVATE_ATTRIBUTES as usize);
        priv_attributes.push(self.serial_number);
        priv_attributes.push(self.binding_number);
        priv_attributes
    }
}

// TODO: this definitely has to be moved somewhere else. It's just a temporary solution
pub async fn obtain_signature(params: &Parameters, attributes: &BandwidthVoucherAttributes, validators: &[Url], verification_key: &VerificationKey) -> Result<Signature, Error> {

    let public_attributes = attributes.get_public_attributes();
    let private_attributes = attributes.get_private_attributes();

    obtain_aggregate_signature(&params, &public_attributes, &private_attributes, validators, verification_key).await
}

pub fn prepare_for_spending(
    raw_identity: &[u8],
    signature: &Signature,
    attributes: &BandwidthVoucherAttributes,
    verification_key: &VerificationKey,
) -> Result<Credential, Error> {
    let public_attributes = vec![BANDWIDTH_VALUE.to_be_bytes().to_vec()];

    let params = Parameters::new(TOTAL_ATTRIBUTES)?;

    prepare_credential_for_spending(
        &params,
        public_attributes,
        attributes.serial_number,
        attributes.binding_number,
        signature,
        verification_key,
    )
}
