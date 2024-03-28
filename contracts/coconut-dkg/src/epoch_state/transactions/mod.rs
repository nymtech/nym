// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::{CURRENT_EPOCH, THRESHOLD};
use crate::error::ContractError;
use crate::state::storage::DKG_ADMIN;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage};
use nym_coconut_dkg_common::types::{Epoch, EpochState};

pub use advance_epoch_state::try_advance_epoch_state;

pub mod advance_epoch_state;

fn reset_dkg_state(storage: &mut dyn Storage) -> Result<(), ContractError> {
    THRESHOLD.remove(storage);

    // dealings are preserved in the storage and saved per epoch, so we don't have to do anything about them
    // the same is true for dealer details
    // and epoch progress is reset when new struct is constructed

    Ok(())
}

pub(crate) fn try_initiate_dkg(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to kick start the process
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let epoch = CURRENT_EPOCH.load(deps.storage)?;
    if !matches!(epoch.state, EpochState::WaitingInitialisation) {
        return Err(ContractError::AlreadyInitialised);
    }

    // the first exchange won't involve resharing
    let initial_state = EpochState::PublicKeySubmission { resharing: false };
    let initial_epoch = Epoch::new(initial_state, 0, epoch.time_configuration, env.block.time);
    CURRENT_EPOCH.save(deps.storage, &initial_epoch)?;

    Ok(Response::default())
}

pub(crate) fn try_trigger_reset(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to trigger DKG reset
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;

    // only allow reset when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    let next_epoch = current_epoch.next_reset(env.block.time);
    CURRENT_EPOCH.save(deps.storage, &next_epoch)?;

    reset_dkg_state(deps.storage)?;

    Ok(Response::default())
}

pub(crate) fn try_trigger_resharing(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to trigger DKG resharing
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;

    // only allow resharing when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    let next_epoch = current_epoch.next_resharing(env.block.time);
    CURRENT_EPOCH.save(deps.storage, &next_epoch)?;

    reset_dkg_state(deps.storage)?;

    Ok(Response::default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cw_controllers::AdminError;

    #[test]
    fn initialising_dkg() {
        let mut deps = init_contract();
        let env = mock_env();

        let initial_epoch_info = CURRENT_EPOCH.load(&deps.storage).unwrap();
        assert!(initial_epoch_info.deadline.is_none());

        // can only be executed by the admin
        let res = try_initiate_dkg(deps.as_mut(), env.clone(), mock_info("not an admin", &[]))
            .unwrap_err();
        assert_eq!(ContractError::Admin(AdminError::NotAdmin {}), res);

        let res = try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[]));
        assert!(res.is_ok());

        // can't be initialised more than once
        let res = try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[]))
            .unwrap_err();
        assert_eq!(ContractError::AlreadyInitialised, res);

        // sets the correct epoch data
        let epoch = CURRENT_EPOCH.load(&deps.storage).unwrap();
        assert_eq!(epoch.epoch_id, 0);
        assert_eq!(
            epoch.state,
            EpochState::PublicKeySubmission { resharing: false }
        );
        assert_eq!(
            epoch.time_configuration,
            initial_epoch_info.time_configuration
        );
        assert_eq!(
            epoch.deadline.unwrap(),
            env.block
                .time
                .plus_seconds(epoch.time_configuration.public_key_submission_time_secs)
        );
    }

    #[test]
    fn reset_state() {
        let mut deps = init_contract();

        THRESHOLD.save(deps.as_mut().storage, &42).unwrap();

        reset_dkg_state(deps.as_mut().storage).unwrap();

        assert!(THRESHOLD.may_load(&deps.storage).unwrap().is_none());
    }
}
