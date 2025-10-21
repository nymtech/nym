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
use crate::verification_key_shares::storage::vk_shares;
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
    DEALERS_INDICES.remove(deps.storage, &info.sender);

    // update registration detail and share information for every epoch the current dealer has participated in the protocol
    // ideally, we'd have only updated the current epoch, but the way the contract is constructed
    // forbids that otherwise we'd have introduced inconsistency
    for epoch_id in 0..=epoch.epoch_id {
        if let Some(details) = EPOCH_DEALERS_MAP.may_load(deps.storage, (epoch_id, &info.sender))? {
            EPOCH_DEALERS_MAP.remove(deps.storage, (epoch_id, &info.sender));
            EPOCH_DEALERS_MAP.save(deps.storage, (epoch_id, &transfer_to), &details)?;
        }
        if let Some(vk_share) = vk_shares().may_load(deps.storage, (&info.sender, epoch_id))? {
            vk_shares().remove(deps.storage, (&info.sender, epoch_id))?;
            vk_shares().save(deps.storage, (&transfer_to, epoch_id), &vk_share)?;
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

    let mut contract_share = vk_shares().load(deps.storage, (&info.sender, epoch.epoch_id))?;
    contract_share.announce_address = new_address.clone();
    vk_shares().save(
        deps.storage,
        (&info.sender, epoch.epoch_id),
        &contract_share,
    )?;

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

#[cfg(test)]
#[cfg(feature = "testable-dkg-contract")]
mod tests_with_mock {
    use super::*;
    use crate::testable_dkg_contract::{
        init_contract_tester, init_contract_tester_with_group_members, DkgContractTesterExt,
    };
    use anyhow::Context;
    use cosmwasm_std::testing::message_info;
    use nym_coconut_dkg_common::msg::QueryMsg;
    use nym_coconut_dkg_common::verification_key::PagedVKSharesResponse;
    use nym_contracts_common_testing::{ChainOpts, ContractOpts};

    #[test]
    fn transferring_ownership() -> anyhow::Result<()> {
        let mut contract = init_contract_tester();
        let group_member = contract.random_group_member();

        // sanity check, pre-dkg
        assert!(DEALERS_INDICES
            .may_load(&contract, &group_member)?
            .is_none());
        assert!(EPOCH_DEALERS_MAP
            .may_load(&contract, (0, &group_member))?
            .is_none());

        contract.run_initial_dummy_dkg();
        let old_index = DEALERS_INDICES.load(&contract, &group_member)?;
        let old_details = EPOCH_DEALERS_MAP.load(&contract, (0, &group_member))?;
        let old_share = vk_shares().load(&contract, (&group_member, 0))?;

        let not_group_member = contract.addr_make("not_group_member");
        let (deps, env) = contract.deps_mut_env();
        assert!(try_transfer_ownership(
            deps,
            env,
            message_info(&group_member, &[]),
            not_group_member.to_string()
        )
        .is_err());

        let new_group_member = contract.addr_make("new_group_member");
        contract.add_group_member(new_group_member.clone());
        let (deps, env) = contract.deps_mut_env();
        assert!(try_transfer_ownership(
            deps,
            env.clone(),
            message_info(&group_member, &[]),
            new_group_member.to_string()
        )
        .is_ok());

        // data under old key doesn't exist anymore
        assert!(DEALERS_INDICES
            .may_load(&contract, &group_member)?
            .is_none());
        assert!(EPOCH_DEALERS_MAP
            .may_load(&contract, (0, &group_member))?
            .is_none());
        assert!(vk_shares()
            .may_load(&contract, (&group_member, 0))?
            .is_none());

        let new_index = DEALERS_INDICES.load(&contract, &new_group_member)?;
        let new_details = EPOCH_DEALERS_MAP.load(&contract, (0, &new_group_member))?;
        let new_share = vk_shares().load(&contract, (&new_group_member, 0))?;

        // the underlying info hasn't changed
        assert_eq!(old_index, new_index);
        assert_eq!(old_details, new_details);
        assert_eq!(old_share, new_share);

        assert_eq!(
            OWNERSHIP_TRANSFER_LOG.load(
                &contract,
                (
                    &group_member,
                    env.block.height,
                    env.transaction.unwrap().index
                )
            )?,
            OwnershipTransfer {
                node_index: new_index,
                from: group_member,
                to: new_group_member,
            }
        );

        Ok(())
    }

    #[test]
    fn transferring_ownership_in_next_epochs() -> anyhow::Result<()> {
        let mut contract = init_contract_tester();
        let group_member = contract.random_group_member();

        contract.run_initial_dummy_dkg(); // => epoch 0
        contract.run_reset_dkg(); // => epoch 1

        // LEAVE DKG MEMBERSHIP
        contract.remove_group_member(group_member.clone());
        contract.run_reset_dkg(); // => epoch 2

        // COME BACK
        contract.add_group_member(group_member.clone());
        contract.run_reset_dkg(); // => epoch 3

        let old_index = DEALERS_INDICES.load(&contract, &group_member)?;
        let old_details0 = EPOCH_DEALERS_MAP.load(&contract, (0, &group_member))?;
        let old_details1 = EPOCH_DEALERS_MAP.load(&contract, (1, &group_member))?;
        let old_details2 = EPOCH_DEALERS_MAP.may_load(&contract, (2, &group_member))?;
        assert!(old_details2.is_none());

        let old_details3 = EPOCH_DEALERS_MAP.load(&contract, (3, &group_member))?;

        // sanity check because we haven't changed our registration details:
        assert_eq!(old_details0, old_details1);
        assert_eq!(old_details1, old_details3);

        let new_group_member = contract.addr_make("new_group_member");
        contract.add_group_member(new_group_member.clone());
        let (deps, env) = contract.deps_mut_env();
        assert!(try_transfer_ownership(
            deps,
            env.clone(),
            message_info(&group_member, &[]),
            new_group_member.to_string()
        )
        .is_ok());

        // data under old key doesn't exist anymore
        assert!(DEALERS_INDICES
            .may_load(&contract, &group_member)?
            .is_none());
        assert!(EPOCH_DEALERS_MAP
            .may_load(&contract, (0, &group_member))?
            .is_none());
        assert!(EPOCH_DEALERS_MAP
            .may_load(&contract, (1, &group_member))?
            .is_none());
        assert!(EPOCH_DEALERS_MAP
            .may_load(&contract, (2, &group_member))?
            .is_none());
        assert!(EPOCH_DEALERS_MAP
            .may_load(&contract, (3, &group_member))?
            .is_none());

        let new_index = DEALERS_INDICES.load(&contract, &new_group_member)?;
        let new_details0 = EPOCH_DEALERS_MAP.load(&contract, (0, &new_group_member))?;
        let new_details1 = EPOCH_DEALERS_MAP.load(&contract, (1, &new_group_member))?;
        let new_details2 = EPOCH_DEALERS_MAP.may_load(&contract, (2, &new_group_member))?;
        let new_details3 = EPOCH_DEALERS_MAP.load(&contract, (3, &new_group_member))?;

        // the underlying info hasn't changed
        assert_eq!(old_index, new_index);
        assert_eq!(old_details0, new_details0);
        assert_eq!(old_details1, new_details1);
        assert_eq!(old_details2, new_details2);
        assert_eq!(old_details3, new_details3);

        assert_eq!(
            OWNERSHIP_TRANSFER_LOG.load(
                &contract,
                (
                    &group_member,
                    env.block.height,
                    env.transaction.unwrap().index
                )
            )?,
            OwnershipTransfer {
                node_index: new_index,
                from: group_member,
                to: new_group_member,
            }
        );

        Ok(())
    }

    #[test]
    fn updating_announce_address() -> anyhow::Result<()> {
        let mut contract = init_contract_tester();
        let group_member = contract.random_group_member();

        contract.run_initial_dummy_dkg(); // => epoch 0
        contract.run_reset_dkg(); // => epoch 1

        // LEAVE DKG MEMBERSHIP
        contract.remove_group_member(group_member.clone());
        contract.run_reset_dkg(); // => epoch 2

        // COME BACK
        contract.add_group_member(group_member.clone());
        contract.run_reset_dkg(); // => epoch 3

        let old_details0 = EPOCH_DEALERS_MAP.load(&contract, (0, &group_member))?;
        let old_details1 = EPOCH_DEALERS_MAP.load(&contract, (1, &group_member))?;
        let old_details2 = EPOCH_DEALERS_MAP.may_load(&contract, (2, &group_member))?;
        assert!(old_details2.is_none());
        let old_details3 = EPOCH_DEALERS_MAP.load(&contract, (3, &group_member))?;

        // sanity check because we haven't changed our registration details:
        assert_eq!(old_details0, old_details1);
        assert_eq!(old_details1, old_details3);

        let new_address = "https://new-address.com".to_string();
        try_update_announce_address(
            contract.deps_mut(),
            message_info(&group_member, &[]),
            new_address.clone(),
        )?;

        let new_details0 = EPOCH_DEALERS_MAP.load(&contract, (0, &group_member))?;
        let new_details1 = EPOCH_DEALERS_MAP.load(&contract, (1, &group_member))?;
        let new_details2 = EPOCH_DEALERS_MAP.may_load(&contract, (2, &group_member))?;
        assert!(new_details2.is_none());
        let new_details3 = EPOCH_DEALERS_MAP.load(&contract, (3, &group_member))?;

        // old epoch data is unchanged
        assert_eq!(old_details0, new_details0);
        assert_eq!(old_details1, new_details1);
        assert_eq!(old_details2, new_details2);

        // most recent entry is updated
        assert_eq!(new_details3.announce_address, new_address);

        Ok(())
    }

    #[test]
    fn updating_announce_address_updates_vk_shares() -> anyhow::Result<()> {
        let mut contract = init_contract_tester_with_group_members(3);
        let group_member = contract.random_group_member();

        contract.run_initial_dummy_dkg(); // => epoch 0
        contract.run_reset_dkg(); // => epoch 1

        // LEAVE DKG MEMBERSHIP
        contract.remove_group_member(group_member.clone());
        contract.run_reset_dkg(); // => epoch 2

        // COME BACK
        contract.add_group_member(group_member.clone());
        contract.run_reset_dkg(); // => epoch 3

        let old_address = EPOCH_DEALERS_MAP
            .load(&contract, (3, &group_member))?
            .announce_address;

        let old_share0 = vk_shares().load(&contract, (&group_member, 0))?;
        let old_share1 = vk_shares().load(&contract, (&group_member, 1))?;
        let old_share2 = vk_shares().may_load(&contract, (&group_member, 2))?;
        assert!(old_share2.is_none());
        let old_share3 = vk_shares().may_load(&contract, (&group_member, 3))?;
        assert!(old_share3.is_some());

        let new_address = "https://new-address.com".to_string();
        try_update_announce_address(
            contract.deps_mut(),
            message_info(&group_member, &[]),
            new_address.clone(),
        )?;

        let new_share0 = vk_shares().load(&contract, (&group_member, 0))?;
        let new_share1 = vk_shares().load(&contract, (&group_member, 1))?;
        let new_share2 = vk_shares().may_load(&contract, (&group_member, 2))?;
        assert!(new_share2.is_none());
        let new_share3 = vk_shares().load(&contract, (&group_member, 3))?;

        // old epoch data is unchanged
        assert_eq!(old_share0, new_share0);
        assert_eq!(old_share1, new_share1);
        assert_eq!(old_share2, new_share2);

        // most recent entry is updated
        assert_eq!(new_share3.announce_address, new_address);

        // finally an integration check against query endpoint
        let epoch0_shares: PagedVKSharesResponse =
            contract.query(&QueryMsg::GetVerificationKeys {
                epoch_id: 0,
                limit: None,
                start_after: None,
            })?;
        assert_eq!(epoch0_shares.shares.len(), 3);

        let member_share = epoch0_shares
            .shares
            .iter()
            .find(|s| s.owner == group_member)
            .context("failed to find member's share")?;
        assert_eq!(member_share.announce_address, old_address);

        let epoch0_shares: PagedVKSharesResponse =
            contract.query(&QueryMsg::GetVerificationKeys {
                epoch_id: 3,
                limit: None,
                start_after: None,
            })?;
        assert_eq!(epoch0_shares.shares.len(), 3);

        let member_share = epoch0_shares
            .shares
            .iter()
            .find(|s| s.owner == group_member)
            .context("failed to find member's share")?;
        assert_eq!(member_share.announce_address, new_address);

        Ok(())
    }
}
