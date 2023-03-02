// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::storage as signing_storage;
use crate::support::helpers::decode_ed25519_identity_key;
use cosmwasm_std::{Addr, Deps};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{construct_family_creation_sign_payload, IdentityKeyRef};
use nym_contracts_common::signing::{MessageSignature, Verifier};

pub(crate) fn verify_family_creation_signature(
    deps: Deps<'_>,
    sender: Addr,
    proxy: Option<Addr>,
    label: String,
    public_key: IdentityKeyRef,
    signature: MessageSignature,
) -> Result<(), MixnetContractError> {
    // recover the public key
    let public_key = decode_ed25519_identity_key(public_key)?;

    // reconstruct the payload
    let nonce = signing_storage::get_signing_nonce(deps.storage, sender.clone())?;
    let msg = construct_family_creation_sign_payload(nonce, sender, proxy, label);

    if deps.api.verify_message(msg, signature, &public_key)? {
        Ok(())
    } else {
        Err(MixnetContractError::InvalidEd25519Signature)
    }
}
