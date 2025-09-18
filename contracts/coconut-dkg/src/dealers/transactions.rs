// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{
    ensure_dealer, get_or_assign_index, is_dealer, save_dealer_details_if_not_a_dealer,
    DEALERS_INDICES, EPOCH_DEALERS_MAP, OWNERSHIP_TRANSFER_LOG,
};
use crate::epoch_state::storage::{load_current_epoch, save_epoch};
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::STATE;
use crate::Dealer;
use cosmwasm_std::{Deps, DepsMut, Env, Event, MessageInfo, Response};
use nym_coconut_dkg_common::dealer::{DealerRegistrationDetails, OwnershipTransfer};
use nym_coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, EpochState};

fn ensure_group_member(deps: Deps, dealer: Dealer) -> Result<(), ContractError> {
    let state = STATE.load(deps.storage)?;

    state
        .group_addr
        .is_voting_member(&deps.querier, dealer, None)?
        .ok_or(ContractError::Unauthorized {})?;

    Ok(())
}

// future optimisation:
// for a recurring dealer just let it refresh the keys without having to do all the storage operations
pub fn try_add_dealer(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    bte_key_with_proof: EncodedBTEPublicKeyWithProof,
    identity_key: String,
    announce_address: String,
    resharing: bool,
) -> Result<Response, ContractError> {
    let epoch = load_current_epoch(deps.storage)?;
    check_epoch_state(deps.storage, EpochState::PublicKeySubmission { resharing })?;

    // make sure this potential dealer actually belong to the group
    ensure_group_member(deps.as_ref(), &info.sender)?;

    let node_index = get_or_assign_index(deps.storage, &info.sender)?;

    // save the dealer into the storage (if it hasn't already been saved)
    let dealer_details = DealerRegistrationDetails {
        bte_public_key_with_proof: bte_key_with_proof,
        ed25519_identity: identity_key,
        announce_address,
    };
    save_dealer_details_if_not_a_dealer(
        deps.storage,
        &info.sender,
        epoch.epoch_id,
        dealer_details,
    )?;

    // check if it's a resharing dealer
    // SAFETY: resharing isn't allowed on 0th epoch
    #[allow(clippy::expect_used)]
    let is_resharing_dealer = resharing
        && is_dealer(
            deps.storage,
            &info.sender,
            epoch
                .epoch_id
                .checked_sub(1)
                .expect("epoch invariant broken: resharing during 0th epoch"),
        );

    // increment the number of registered dealers
    {
        let current_epoch = load_current_epoch(deps.storage)?;
        let mut updated_epoch = current_epoch;
        updated_epoch.state_progress.registered_dealers += 1;

        if is_resharing_dealer {
            updated_epoch.state_progress.registered_resharing_dealers += 1;
        }
        save_epoch(deps.storage, env.block.height, &updated_epoch)?;
    }

    Ok(Response::new().add_attribute("node_index", node_index.to_string()))
}

pub fn try_transfer_ownership(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    transfer_to: String,
) -> Result<Response, ContractError> {
    let transfer_to = deps.api.addr_validate(&transfer_to)?;

    let epoch = load_current_epoch(deps.storage)?;

    // make sure we're not mid-exchange
    check_epoch_state(deps.storage, EpochState::InProgress)?;

    // make sure the requester is actually a dealer for this epoch
    ensure_dealer(deps.storage, &info.sender, epoch.epoch_id)?;

    // make sure the new target dealer actually belong to the group
    ensure_group_member(deps.as_ref(), &transfer_to)?;

    // update the index information
    let current_index = DEALERS_INDICES.load(deps.storage, &info.sender)?;
    DEALERS_INDICES.save(deps.storage, &transfer_to, &current_index)?;

    // update registration detail for every epoch the current dealer has participated in the protocol
    // ideally, we'd have only updated the current epoch, but the way the contract is constructed
    // forbids that otherwise we'd have introduced inconsistency
    for epoch_id in 0..=epoch.epoch_id {
        if let Some(details) = EPOCH_DEALERS_MAP.may_load(deps.storage, (epoch_id, &info.sender))? {
            EPOCH_DEALERS_MAP.remove(deps.storage, (epoch_id, &info.sender));
            EPOCH_DEALERS_MAP.save(deps.storage, (epoch_id, &transfer_to), &details)?;
        }
    }

    let Some(transaction_info) = env.transaction else {
        return Err(ContractError::ExecutedOutsideTransaction);
    };

    // save information about the transfer for more convenient history rebuilding
    OWNERSHIP_TRANSFER_LOG.save(
        deps.storage,
        (&info.sender, env.block.height, transaction_info.index),
        &OwnershipTransfer {
            node_index: current_index,
            from: info.sender.clone(),
            to: transfer_to.clone(),
        },
    )?;

    Ok(Response::new().add_event(
        Event::new("dkg-ownership-transfer")
            .add_attribute("from", info.sender)
            .add_attribute("to", transfer_to)
            .add_attribute("node_index", current_index.to_string()),
    ))
}

pub fn try_update_announce_address(
    deps: DepsMut<'_>,
    info: MessageInfo,
    new_address: String,
) -> Result<Response, ContractError> {
    let epoch = load_current_epoch(deps.storage)?;

    // make sure we're not mid-exchange
    check_epoch_state(deps.storage, EpochState::InProgress)?;

    // make sure the requester is actually a dealer for this epoch
    ensure_dealer(deps.storage, &info.sender, epoch.epoch_id)?;

    let mut details = EPOCH_DEALERS_MAP.load(deps.storage, (epoch.epoch_id, &info.sender))?;
    let old_address = details.announce_address;

    details.announce_address = new_address.clone();
    EPOCH_DEALERS_MAP.save(deps.storage, (epoch.epoch_id, &info.sender), &details)?;

    Ok(Response::new().add_event(
        Event::new("dkg-announce-address-update")
            .add_attribute("dealer", info.sender)
            .add_attribute("old_address", old_address)
            .add_attribute("new_address", new_address),
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::transactions::{try_advance_epoch_state, try_initiate_dkg};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{add_fixture_dealer, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{message_info, mock_env};
    use cosmwasm_std::Addr;
    use nym_coconut_dkg_common::types::TimeConfiguration;

    #[test]
    fn invalid_state() {
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
        let bte_key_with_proof = String::from("bte_key_with_proof");
        let identity = String::from("identity");
        let announce_address = String::from("localhost:8000");

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);

        add_fixture_dealer(deps.as_mut());
        try_advance_epoch_state(deps.as_mut(), env.clone()).unwrap();

        let ret = try_add_dealer(
            deps.as_mut(),
            env,
            info,
            bte_key_with_proof,
            identity,
            announce_address,
            false,
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::DealingExchange { resharing: false }.to_string(),
                expected_state: EpochState::PublicKeySubmission { resharing: false }.to_string(),
            }
        );
    }
}
