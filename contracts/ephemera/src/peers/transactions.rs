// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::peers::storage::PEERS;
use cosmwasm_std::{DepsMut, MessageInfo, Response};
use nym_ephemera_common::types::JsonPeerInfo;

pub fn try_register_peer(
    deps: DepsMut<'_>,
    info: MessageInfo,
    peer_info: JsonPeerInfo,
) -> Result<Response, ContractError> {
    if PEERS.may_load(deps.storage, info.sender.clone())?.is_none() {
        PEERS.save(deps.storage, info.sender, &peer_info)?;
    } else {
        return Err(ContractError::AlreadyRegistered);
    }
    Ok(Default::default())
}
