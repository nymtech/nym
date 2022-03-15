// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_bandwidth_contract::events::{
    DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_ENCRYPTION_KEY, DEPOSIT_INFO, DEPOSIT_VALUE,
    DEPOSIT_VERIFICATION_KEY,
};
use coconut_interface::BlindSignRequestBody;
use credentials::coconut::bandwidth::BandwidthVoucher;
use crypto::asymmetric::encryption;
use crypto::asymmetric::identity::{self, Signature};
use validator_client::nymd::tx::Hash;
use validator_client::nymd::NymdClient;

use super::error::{CoconutError, Result};
use crate::config::DEFAULT_LOCAL_VALIDATOR;

pub async fn extract_encryption_key(
    blind_sign_request_body: &BlindSignRequestBody,
) -> Result<encryption::PublicKey> {
    let blind_sign_request = blind_sign_request_body.blind_sign_request();
    let public_attributes = blind_sign_request_body.public_attributes();
    let public_attributes_plain = blind_sign_request_body.public_attributes_plain();

    if !BandwidthVoucher::verify_against_plain(&public_attributes, public_attributes_plain) {
        return Err(CoconutError::InconsistentPublicAttributes);
    }

    let tx_hash = blind_sign_request_body.tx_hash();
    let mut message = blind_sign_request.to_bytes();
    message.extend_from_slice(tx_hash.as_bytes());

    let signature = Signature::from_base58_string(blind_sign_request_body.signature())?;
    let tx_hash = tx_hash
        .parse::<Hash>()
        .map_err(|_| CoconutError::TxHashParseError)?;

    let nymd_client = NymdClient::connect(DEFAULT_LOCAL_VALIDATOR, None, None, None)?;

    let tx = nymd_client.get_tx(tx_hash).await?;
    let attributes: &Vec<_> = tx
        .tx_result
        .events
        .iter()
        .find(|event| event.type_str == format!("wasm-{}", DEPOSITED_FUNDS_EVENT_TYPE))
        .ok_or(CoconutError::InvalidTx)?
        .attributes
        .as_ref();

    let deposit_value = attributes
        .iter()
        .find(|tag| tag.key.as_ref() == DEPOSIT_VALUE)
        .ok_or(CoconutError::InvalidTx)?
        .value
        .as_ref();
    let deposit_value_plain = public_attributes_plain
        .get(0)
        .cloned()
        .unwrap_or(String::new());
    if deposit_value != deposit_value_plain {
        return Err(CoconutError::DifferentPublicAttributes(
            deposit_value.to_string(),
            deposit_value_plain.to_string(),
        ));
    }

    let deposit_info = attributes
        .iter()
        .find(|tag| tag.key.as_ref() == DEPOSIT_INFO)
        .ok_or(CoconutError::InvalidTx)?
        .value
        .as_ref();
    let deposit_info_plain = public_attributes_plain
        .get(1)
        .cloned()
        .unwrap_or(String::new());
    if deposit_info != deposit_info_plain {
        return Err(CoconutError::DifferentPublicAttributes(
            deposit_info.to_string(),
            deposit_info_plain.to_string(),
        ));
    }

    let verification_key = identity::PublicKey::from_base58_string(
        attributes
            .iter()
            .find(|tag| tag.key.as_ref() == DEPOSIT_VERIFICATION_KEY)
            .ok_or(CoconutError::InvalidTx)?
            .value
            .as_ref(),
    )?;
    verification_key.verify(&message, &signature)?;

    let encryption_key = encryption::PublicKey::from_base58_string(
        attributes
            .iter()
            .find(|tag| tag.key.as_ref() == DEPOSIT_ENCRYPTION_KEY)
            .ok_or(CoconutError::InvalidTx)?
            .value
            .as_ref(),
    )?;

    Ok(encryption_key)
}
