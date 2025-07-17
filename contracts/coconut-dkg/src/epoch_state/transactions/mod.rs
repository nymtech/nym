// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::{load_current_epoch, save_epoch, THRESHOLD};
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

    let epoch = load_current_epoch(deps.storage)?;
    if !matches!(epoch.state, EpochState::WaitingInitialisation) {
        return Err(ContractError::AlreadyInitialised);
    }

    // the first exchange won't involve resharing
    let initial_state = EpochState::PublicKeySubmission { resharing: false };
    let initial_epoch = Epoch::new(initial_state, 0, epoch.time_configuration, env.block.time);
    save_epoch(deps.storage, &initial_epoch)?;

    Ok(Response::default())
}

pub(crate) fn try_trigger_reset(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // only the admin is allowed to trigger DKG reset
    DKG_ADMIN.assert_admin(deps.as_ref(), &info.sender)?;
    let current_epoch = load_current_epoch(deps.storage)?;

    // only allow reset when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    let next_epoch = current_epoch.next_reset(env.block.time);
    save_epoch(deps.storage, &next_epoch)?;

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
    let current_epoch = load_current_epoch(deps.storage)?;

    // only allow resharing when the DKG exchange isn't in progress
    if !current_epoch.state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    let next_epoch = current_epoch.next_resharing(env.block.time);
    save_epoch(deps.storage, &next_epoch)?;

    reset_dkg_state(deps.storage)?;

    Ok(Response::default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::storage::load_current_epoch;
    use crate::support::tests::helpers::{init_contract, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::Addr;
    use cw_controllers::AdminError;

    #[test]
    fn initialising_dkg() {
        let mut deps = init_contract();
        let env = mock_env();

        let initial_epoch_info = load_current_epoch(&deps.storage).unwrap();
        assert!(initial_epoch_info.deadline.is_none());

        let not_admin = deps.api.addr_make("not an admin");
        // can only be executed by the admin
        let res = try_initiate_dkg(deps.as_mut(), env.clone(), message_info(&not_admin, &[]))
            .unwrap_err();
        assert_eq!(ContractError::Admin(AdminError::NotAdmin {}), res);

        let res = try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        );
        assert!(res.is_ok());

        // can't be initialised more than once
        let res = try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap_err();
        assert_eq!(ContractError::AlreadyInitialised, res);

        // sets the correct epoch data
        let epoch = load_current_epoch(&deps.storage).unwrap();
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
