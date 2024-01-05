// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::epoch_state::storage::INITIAL_REPLACEMENT_DATA;
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::storage::STATE;
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};
use nym_coconut_dkg_common::types::{DealerDetails, EncodedBTEPublicKeyWithProof, EpochState};

// currently we only require that
// a) it's part of the signer group
// b) it isn't already a dealer
fn verify_dealer(deps: DepsMut<'_>, dealer: &Addr, resharing: bool) -> Result<(), ContractError> {
    if dealers_storage::current_dealers()
        .may_load(deps.storage, dealer)?
        .is_some()
    {
        return Err(ContractError::AlreadyADealer);
    }
    let state = STATE.load(deps.storage)?;

    let height = if resharing {
        Some(INITIAL_REPLACEMENT_DATA.load(deps.storage)?.initial_height)
    } else {
        None
    };
    state
        .group_addr
        .is_voting_member(&deps.querier, dealer, height)?
        .ok_or(ContractError::Unauthorized {})?;

    Ok(())
}

pub fn try_add_dealer(
    mut deps: DepsMut<'_>,
    info: MessageInfo,
    bte_key_with_proof: EncodedBTEPublicKeyWithProof,
    identity_key: String,
    announce_address: String,
    resharing: bool,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::PublicKeySubmission { resharing })?;

    verify_dealer(deps.branch(), &info.sender, resharing)?;

    // if it was already a dealer in the past, assign the same node index
    let node_index = if let Some(prior_details) =
        dealers_storage::past_dealers().may_load(deps.storage, &info.sender)?
    {
        // since this dealer is going to become active now, remove it from the past dealers
        dealers_storage::past_dealers().replace(
            deps.storage,
            &info.sender,
            None,
            Some(&prior_details),
        )?;
        prior_details.assigned_index
    } else {
        dealers_storage::next_node_index(deps.storage)?
    };

    // save the dealer into the storage
    let dealer_details = DealerDetails {
        address: info.sender.clone(),
        bte_public_key_with_proof: bte_key_with_proof,
        ed25519_identity: identity_key,
        announce_address,
        assigned_index: node_index,
    };
    dealers_storage::current_dealers().save(deps.storage, &info.sender, &dealer_details)?;

    Ok(Response::new().add_attribute("node_index", node_index.to_string()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dealers::storage::current_dealers;
    use crate::epoch_state::transactions::{advance_epoch_state, try_initiate_dkg};
    use crate::support::tests::fixtures::dealer_details_fixture;
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{add_fixture_dealer, ADMIN_ADDRESS, GROUP_MEMBERS};
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cw4::Member;
    use nym_coconut_dkg_common::types::{InitialReplacementData, TimeConfiguration};
    use rusty_fork::rusty_fork_test;

    rusty_fork_test! {
        #[test]
        fn verification() {
            let mut deps = helpers::init_contract();
            let new_dealer = Addr::unchecked("new_dealer");
            let details1 = dealer_details_fixture(1);
            let details2 = dealer_details_fixture(2);
            let details3 = dealer_details_fixture(3);
            current_dealers()
                .save(deps.as_mut().storage, &details1.address, &details1)
                .unwrap();
            let err = verify_dealer(deps.as_mut(), &details1.address, false).unwrap_err();
            assert_eq!(err, ContractError::AlreadyADealer);

            INITIAL_REPLACEMENT_DATA
                .save(
                    deps.as_mut().storage,
                    &InitialReplacementData {
                        initial_dealers: vec![details1.address, details2.address, details3.address],
                        initial_height: 1,
                    },
                )
                .unwrap();
            let err = verify_dealer(deps.as_mut(), &new_dealer, false).unwrap_err();
            assert_eq!(err, ContractError::Unauthorized);

            GROUP_MEMBERS.lock().unwrap().push((
                Member {
                    addr: new_dealer.to_string(),
                    weight: 10,
                },
                2,
            ));
            verify_dealer(deps.as_mut(), &new_dealer, false).unwrap();

            let err = verify_dealer(deps.as_mut(), &new_dealer, true).unwrap_err();
            assert_eq!(err, ContractError::Unauthorized);

            INITIAL_REPLACEMENT_DATA
                .update::<_, ContractError>(deps.as_mut().storage, |mut data| {
                    data.initial_height = 2;
                    Ok(data)
                })
                .unwrap();
            verify_dealer(deps.as_mut(), &new_dealer, true).unwrap();
        }
    }

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
        advance_epoch_state(deps.as_mut(), env).unwrap();

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
