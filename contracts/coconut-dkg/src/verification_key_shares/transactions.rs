// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::BLOCK_TIME_FOR_VERIFICATION_SECS;
use crate::dealers::storage as dealers_storage;
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::{MULTISIG, STATE};
use crate::verification_key_shares::storage::VK_SHARES;
use coconut_dkg_common::types::EpochState;
use coconut_dkg_common::verification_key::{to_cosmos_msg, ContractVKShare, VerificationKeyShare};
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};

pub fn try_commit_verification_key_share(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    share: VerificationKeyShare,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::VerificationKeySubmission)?;
    // ensure the sender is a dealer
    let details = dealers_storage::current_dealers()
        .load(deps.storage, &info.sender)
        .map_err(|_| ContractError::NotADealer)?;
    if VK_SHARES.may_load(deps.storage, &info.sender)?.is_some() {
        return Err(ContractError::AlreadyCommitted {
            commitment: String::from("verification key share"),
        });
    }

    let data = ContractVKShare {
        share,
        node_index: details.assigned_index,
        announce_address: details.announce_address,
        owner: info.sender.clone(),
        verified: false,
    };
    VK_SHARES.save(deps.storage, &info.sender, &data)?;

    let msg = to_cosmos_msg(
        info.sender,
        env.contract.address.to_string(),
        STATE.load(deps.storage)?.multisig_addr.to_string(),
        env.block
            .time
            .plus_seconds(BLOCK_TIME_FOR_VERIFICATION_SECS),
    )?;

    Ok(Response::new().add_message(msg))
}

pub fn try_verify_verification_key_share(
    deps: DepsMut<'_>,
    info: MessageInfo,
    owner: Addr,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::VerificationKeyFinalization)?;
    MULTISIG.assert_admin(deps.as_ref(), &info.sender)?;
    VK_SHARES.update(deps.storage, &owner, |vk_share| {
        vk_share
            .map(|mut share| {
                share.verified = true;
                share
            })
            .ok_or(ContractError::NoCommitForOwner {
                owner: owner.to_string(),
            })
    })?;

    Ok(Response::default())
}
