// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::storage as signing_storage;
use crate::support::helpers::decode_ed25519_identity_key;
use cosmwasm_std::{Addr, Coin, Deps};
use mixnet_contract_common::construct_generic_node_bonding_payload;
use mixnet_contract_common::error::MixnetContractError;
use nym_contracts_common::signing::Verifier;
use nym_contracts_common::signing::{MessageSignature, SigningPurpose};
use nym_contracts_common::IdentityKeyRef;
use serde::Serialize;

/// Verifies the bonding signature on either a legacy mixnode, legacy gateway or a nym-node.
pub(crate) fn verify_bonding_signature<T>(
    deps: Deps<'_>,
    sender: Addr,
    identity_key: IdentityKeyRef,
    pledge: Coin,
    message: T,
    signature: MessageSignature,
) -> Result<(), MixnetContractError>
where
    T: SigningPurpose + Serialize,
{
    // recover the public key
    let public_key = decode_ed25519_identity_key(identity_key)?;

    // reconstruct the payload
    let nonce = signing_storage::get_signing_nonce(deps.storage, sender.clone())?;
    let msg = construct_generic_node_bonding_payload(nonce, sender, pledge, message);

    if deps.api.verify_message(msg, signature, &public_key)? {
        Ok(())
    } else {
        Err(MixnetContractError::InvalidEd25519Signature)
    }
}
