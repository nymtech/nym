// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::dealings::storage::DEALING_COMMITMENTS;
use crate::epoch::storage as epoch_storage;
use crate::ContractError;
use coconut_dkg_common::types::ContractSafeCommitment;
use cosmwasm_std::{DepsMut, MessageInfo, Response};

pub fn try_commit_dealing(
    deps: DepsMut<'_>,
    info: MessageInfo,
    epoch_id: u32,
    commitment: ContractSafeCommitment,
) -> Result<Response, ContractError> {
    let current_epoch = epoch_storage::current_epoch(deps.storage)?;
    // TODO: check if we're in correct epoch state (i.e. current_epoch.state)

    // ensure the sender is a dealer for the current epoch
    if dealers_storage::current_dealers()
        .may_load(deps.storage, &info.sender)?
        .is_none()
    {
        return Err(ContractError::NotADealer);
    }

    // make sure the dealer wants to submit commitment for THIS epoch
    if current_epoch.id != epoch_id {
        return Err(ContractError::MismatchedEpoch {
            current: current_epoch.id,
            request_for: epoch_id,
        });
    }

    // check if this dealer has already committed to a dealing
    // (we don't want to allow overwriting it as some receivers might already be using the commitment)
    let storage_key = (epoch_id, &info.sender);

    if DEALING_COMMITMENTS.has(deps.storage, storage_key) {
        return Err(ContractError::AlreadyCommitted);
    }

    DEALING_COMMITMENTS.save(deps.storage, storage_key, &commitment)?;

    Ok(Response::new())
}
