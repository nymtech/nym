// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::storage as signing_storage;
use crate::support::helpers::decode_ed25519_identity_key;
use cosmwasm_std::{Addr, Coin, Deps};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{construct_gateway_bonding_sign_payload, Gateway};
use nym_contracts_common::signing::MessageSignature;
use nym_contracts_common::signing::Verifier;

pub(crate) fn verify_gateway_bonding_signature(
    deps: Deps<'_>,
    sender: Addr,
    pledge: Coin,
    gateway: Gateway,
    signature: MessageSignature,
) -> Result<(), MixnetContractError> {
    // recover the public key
    let public_key = decode_ed25519_identity_key(&gateway.identity_key)?;

    // reconstruct the payload
    let nonce = signing_storage::get_signing_nonce(deps.storage, sender.clone())?;
    let msg = construct_gateway_bonding_sign_payload(nonce, sender, pledge, gateway);

    if deps.api.verify_message(msg, signature, &public_key)? {
        Ok(())
    } else {
        Err(MixnetContractError::InvalidEd25519Signature)
    }
}
