// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::BLOCK_TIME_FOR_VERIFICATION_SECS;
use crate::dealers::storage::get_dealer_details;
use crate::epoch_state::storage::{load_current_epoch, save_epoch};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::{MULTISIG, STATE};
use crate::verification_key_shares::storage::vk_shares;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use nym_coconut_dkg_common::types::EpochState;
use nym_coconut_dkg_common::verification_key::{
    to_cosmos_msg, ContractVKShare, VerificationKeyShare,
};

pub fn try_commit_verification_key_share(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    share: VerificationKeyShare,
    resharing: bool,
) -> Result<Response, ContractError> {
    check_epoch_state(
        deps.storage,
        EpochState::VerificationKeySubmission { resharing },
    )?;
    let mut epoch = load_current_epoch(deps.storage)?;
    let epoch_id = epoch.epoch_id;

    let details = get_dealer_details(deps.storage, &info.sender, epoch_id)?;
    if vk_shares()
        .may_load(deps.storage, (&info.sender, epoch_id))?
        .is_some()
    {
        return Err(ContractError::AlreadyCommitted {
            commitment: String::from("verification key share"),
        });
    }

    let data = ContractVKShare {
        share,
        node_index: details.assigned_index,
        announce_address: details.announce_address,
        owner: info.sender.clone(),
        epoch_id: load_current_epoch(deps.storage)?.epoch_id,
        verified: false,
    };
    vk_shares().save(deps.storage, (&info.sender, epoch_id), &data)?;

    let msg = to_cosmos_msg(
        info.sender,
        resharing,
        env.contract.address.to_string(),
        STATE.load(deps.storage)?.multisig_addr.to_string(),
        // TODO: make this value configurable
        env.block
            .time
            .plus_seconds(BLOCK_TIME_FOR_VERIFICATION_SECS),
    )?;

    epoch.state_progress.submitted_key_shares += 1;
    save_epoch(deps.storage, env.block.height, &epoch)?;

    Ok(Response::new().add_message(msg))
}

pub fn try_verify_verification_key_share(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    owner: String,
    resharing: bool,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_validate(&owner)?;

    check_epoch_state(
        deps.storage,
        EpochState::VerificationKeyFinalization { resharing },
    )?;
    let mut epoch = load_current_epoch(deps.storage)?;
    let epoch_id = epoch.epoch_id;

    MULTISIG.assert_admin(deps.as_ref(), &info.sender)?;
    vk_shares().update(deps.storage, (&owner, epoch_id), |vk_share| {
        vk_share
            .map(|mut share| {
                share.verified = true;
                share
            })
            .ok_or(ContractError::NoCommitForOwner {
                owner: owner.to_string(),
            })
    })?;

    epoch.state_progress.verified_keys += 1;
    save_epoch(deps.storage, env.block.height, &epoch)?;

    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch_state::transactions::{try_advance_epoch_state, try_initiate_dkg};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{
        add_current_dealer, add_fixture_dealer, ADMIN_ADDRESS, MULTISIG_CONTRACT,
    };
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::Addr;
    use cw_controllers::AdminError;
    use nym_coconut_dkg_common::dealer::DealerDetails;
    use nym_coconut_dkg_common::types::TimeConfiguration;

    #[test]
    fn current_epoch_id() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        let info = message_info(&deps.api.addr_make("requester"), &[]);
        let share = "share".to_string();

        add_fixture_dealer(deps.as_mut());
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().dealing_exchange_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let dealer = deps.api.addr_make("requester");
        let announce_address = String::from("localhost");
        let dealer_details = DealerDetails {
            address: dealer.clone(),
            bte_public_key_with_proof: String::new(),
            ed25519_identity: String::new(),
            announce_address: announce_address.clone(),
            assigned_index: 1,
        };
        add_current_dealer(deps.as_mut(), &dealer_details);

        try_commit_verification_key_share(deps.as_mut(), env, info.clone(), share.clone(), false)
            .unwrap();
        let vk_share = vk_shares().load(&deps.storage, (&info.sender, 0)).unwrap();
        assert_eq!(
            vk_share,
            ContractVKShare {
                share,
                announce_address,
                node_index: 1,
                owner: dealer,
                epoch_id: 0,
                verified: false,
            }
        );
    }

    #[test]
    fn commit_vk_share() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        let info = message_info(&deps.api.addr_make("requester"), &[]);
        let share = "share".to_string();

        let ret = try_commit_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            share.clone(),
            false,
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::PublicKeySubmission { resharing: false }.to_string(),
                expected_state: EpochState::VerificationKeySubmission { resharing: false }
                    .to_string()
            }
        );
        add_fixture_dealer(deps.as_mut());
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().dealing_exchange_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        let ret = try_commit_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            share.clone(),
            false,
        )
        .unwrap_err();
        assert_eq!(ret, ContractError::NotADealer { epoch_id: 0 });

        let dealer = deps.api.addr_make("requester");
        let dealer_details = DealerDetails {
            address: dealer.clone(),
            bte_public_key_with_proof: String::new(),
            ed25519_identity: String::new(),
            announce_address: String::new(),
            assigned_index: 1,
        };
        add_current_dealer(deps.as_mut(), &dealer_details);

        try_commit_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            share.clone(),
            false,
        )
        .unwrap();

        let ret =
            try_commit_verification_key_share(deps.as_mut(), env, info, share, false).unwrap_err();
        assert_eq!(
            ret,
            ContractError::AlreadyCommitted {
                commitment: String::from("verification key share")
            }
        );
    }

    #[test]
    fn invalid_verify_vk_share() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        let info = message_info(&deps.api.addr_make("requester"), &[]);
        let owner = deps.api.addr_make("owner").to_string();
        let multisig_info = message_info(&Addr::unchecked(MULTISIG_CONTRACT), &[]);

        let ret = try_verify_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            owner.clone(),
            false,
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::PublicKeySubmission { resharing: false }.to_string(),
                expected_state: EpochState::VerificationKeyFinalization { resharing: false }
                    .to_string()
            }
        );

        add_fixture_dealer(deps.as_mut());
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().dealing_exchange_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().verification_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().verification_key_validation_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        let ret = try_verify_verification_key_share(
            deps.as_mut(),
            env.clone(),
            info,
            owner.clone(),
            false,
        )
        .unwrap_err();
        assert_eq!(ret, ContractError::Admin(AdminError::NotAdmin {}));

        let ret = try_verify_verification_key_share(
            deps.as_mut(),
            env.clone(),
            multisig_info,
            owner.clone(),
            false,
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::NoCommitForOwner {
                owner: owner.to_string()
            }
        );
    }

    #[test]
    fn verify_vk_share() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        try_initiate_dkg(
            deps.as_mut(),
            env.clone(),
            message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]),
        )
        .unwrap();

        let owner = deps.api.addr_make("owner");
        let info = message_info(&owner, &[]);
        let share = "share".to_string();
        let multisig_info = message_info(&Addr::unchecked(MULTISIG_CONTRACT), &[]);

        add_fixture_dealer(deps.as_mut());
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().dealing_exchange_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        let dealer_details = DealerDetails {
            address: Addr::unchecked(&owner),
            bte_public_key_with_proof: String::new(),
            ed25519_identity: String::new(),
            announce_address: String::new(),
            assigned_index: 1,
        };
        add_current_dealer(deps.as_mut(), &dealer_details);

        try_commit_verification_key_share(deps.as_mut(), env.clone(), info, share, false).unwrap();

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().verification_key_submission_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();
        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().verification_key_validation_time_secs);
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        try_verify_verification_key_share(
            deps.as_mut(),
            env,
            multisig_info,
            owner.to_string(),
            false,
        )
        .unwrap();
    }
}
