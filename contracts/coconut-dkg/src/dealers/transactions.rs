// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{
    get_or_assign_index, is_dealer, save_dealer_details_if_not_a_dealer,
};
use crate::epoch_state::storage::CURRENT_EPOCH;
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::STATE;
use crate::Dealer;
use cosmwasm_std::{Deps, DepsMut, MessageInfo, Response, StdResult};
use nym_coconut_dkg_common::dealer::DealerRegistrationDetails;
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
    info: MessageInfo,
    bte_key_with_proof: EncodedBTEPublicKeyWithProof,
    identity_key: String,
    announce_address: String,
    resharing: bool,
) -> Result<Response, ContractError> {
    let epoch = CURRENT_EPOCH.load(deps.storage)?;
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
    CURRENT_EPOCH.update(deps.storage, |epoch| -> StdResult<_> {
        let mut updated_epoch = epoch;
        updated_epoch.state_progress.registered_dealers += 1;

        if is_resharing_dealer {
            updated_epoch.state_progress.registered_resharing_dealers += 1;
        }

        Ok(updated_epoch)
    })?;

    Ok(Response::new().add_attribute("node_index", node_index.to_string()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::transactions::{try_advance_epoch_state, try_initiate_dkg};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{add_fixture_dealer, ADMIN_ADDRESS};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Addr;
    use nym_coconut_dkg_common::types::TimeConfiguration;

    #[test]
    fn invalid_state() {
        let mut deps = helpers::init_contract();
        let mut env = mock_env();
        try_initiate_dkg(deps.as_mut(), env.clone(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

        let owner = Addr::unchecked("owner");
        let info = mock_info(owner.as_str(), &[]);
        let bte_key_with_proof = String::from("bte_key_with_proof");
        let identity = String::from("identity");
        let announce_address = String::from("localhost:8000");

        env.block.time = env
            .block
            .time
            .plus_seconds(TimeConfiguration::default().public_key_submission_time_secs);

        add_fixture_dealer(deps.as_mut());
        try_advance_epoch_state(deps.as_mut(), env).unwrap();

        let ret = try_add_dealer(
            deps.as_mut(),
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
