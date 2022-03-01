// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::storage;
use coconut_dkg_common::types::EncodedChannelPublicKey;
use cosmwasm_std::{ensure, DepsMut, Env, MessageInfo, Response};
use cosmwasm_std::{Addr, Deps};

fn is_validator(deps: &DepsMut, addr: &Addr) -> bool {
    deps.querier.query_validator(addr).is_ok()
}

pub fn try_submit_public_key(
    deps: DepsMut<'_>,
    info: MessageInfo,
    key: EncodedChannelPublicKey,
) -> Result<Response, ContractError> {
    if !is_validator(&deps, &info.sender) {
        return Err(ContractError::Unauthorized);
    }

    if let Some(node_details) =
        storage::CURRENT_ISSUERS.may_load(deps.storage, info.sender.clone())?
    {
        return Err(ContractError::PublicKeyAlreadySubmitted(
            node_details.public_key,
        ));
    }

    let id = storage::submit_issuer(deps.storage, info.sender, key)?;

    todo!()
}

pub fn try_remove_issuer(deps: DepsMut<'_>, info: MessageInfo) {}

pub fn try_submit_share(
    deps: DepsMut<'_>,
    info: MessageInfo,
    share: (),
) -> Result<Response, ContractError> {
    // if !storage::SECURE_CHANNEL_KEYS.has(deps.storage, info.sender) {
    //     return Err(ContractError::PublicKeyNotKnown);
    // }

    Ok(Default::default())
}

// submit validate vlaidator set?

// if invalid for too long force reshare or something?
