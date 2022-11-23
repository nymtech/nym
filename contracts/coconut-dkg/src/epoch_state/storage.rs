// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::state::ADMIN;
use coconut_dkg_common::types::EpochState;
use cosmwasm_std::{DepsMut, MessageInfo, Response, Storage};
use cw_storage_plus::Item;

pub(crate) const CURRENT_EPOCH_STATE: Item<'_, EpochState> = Item::new("current_epoch_state");

pub(crate) fn current_epoch_state(storage: &dyn Storage) -> Result<EpochState, ContractError> {
    CURRENT_EPOCH_STATE
        .load(storage)
        .map_err(|_| ContractError::EpochNotInitialised)
}

pub(crate) fn advance_epoch_state(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    CURRENT_EPOCH_STATE.update::<_, ContractError>(deps.storage, |mut epoch_state| {
        // TODO: When defaulting to the first state, some action will probably need to be taken on the
        // rest of the contract, as we're starting with a new set of signers
        epoch_state = epoch_state.next().unwrap_or_default();
        Ok(epoch_state)
    })?;
    Ok(Response::default())
}
