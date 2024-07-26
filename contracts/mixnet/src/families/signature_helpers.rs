// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnodes::storage as mixnodes_storage;
use crate::signing::storage as signing_storage;
use crate::support::helpers::decode_ed25519_identity_key;
use cosmwasm_std::Deps;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::{construct_family_join_permit, IdentityKeyRef};
use nym_contracts_common::signing::{MessageSignature, Verifier};

pub(crate) fn verify_family_join_permit(
    deps: Deps<'_>,
    granter: FamilyHead,
    member: IdentityKeyRef,
    signature: MessageSignature,
) -> Result<(), MixnetContractError> {
    // recover the public key
    let public_key = decode_ed25519_identity_key(granter.identity())?;

    // that's kinda a backwards way of getting the granter's nonce, but it works, so ¯\_(ツ)_/¯
    let Some(head_mixnode) = mixnodes_storage::mixnode_bonds()
        .idx
        .identity_key
        .item(deps.storage, granter.identity().to_owned())?
        .map(|record| record.1)
    else {
        return Err(MixnetContractError::FamilyDoesNotExist {
            head: granter.identity().to_string(),
        });
    };
    let nonce = signing_storage::get_signing_nonce(deps.storage, head_mixnode.owner)?;
    let msg = construct_family_join_permit(nonce, granter, member.to_owned());

    if deps.api.verify_message(msg, signature, &public_key)? {
        Ok(())
    } else {
        Err(MixnetContractError::InvalidEd25519Signature)
    }
}
