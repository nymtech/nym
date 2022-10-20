// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::dealings::storage::DEALING_BYTES;
use crate::epoch_state::utils::check_epoch_state;
use crate::ContractError;
use coconut_dkg_common::types::{ContractSafeBytes, EpochState};
use cosmwasm_std::{DepsMut, MessageInfo, Response};

pub fn try_commit_dealing(
    deps: DepsMut<'_>,
    info: MessageInfo,
    dealing_bytes: ContractSafeBytes,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::DealingExchange)?;
    // ensure the sender is a dealer for the current epoch
    if dealers_storage::current_dealers()
        .may_load(deps.storage, &info.sender)?
        .is_none()
    {
        return Err(ContractError::NotADealer);
    }

    // check if this dealer has already committed to a dealing
    // (we don't want to allow overwriting it as some receivers might already be using the commitment)
    if DEALING_BYTES.has(deps.storage, &info.sender) {
        return Err(ContractError::AlreadyCommitted);
    }

    DEALING_BYTES.save(deps.storage, &info.sender, &dealing_bytes)?;

    Ok(Response::new())
}
