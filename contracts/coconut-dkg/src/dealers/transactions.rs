// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage as dealers_storage;
use crate::epoch_state::utils::check_epoch_state;
use crate::error::ContractError;
use crate::state::{State, STATE};
use coconut_dkg_common::types::{DealerDetails, EncodedBTEPublicKeyWithProof, EpochState};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};

// currently we only require that
// a) it's part of the signer group
// b) it isn't already a dealer
fn verify_dealer(deps: DepsMut<'_>, state: &State, dealer: &Addr) -> Result<(), ContractError> {
    if dealers_storage::current_dealers()
        .may_load(deps.storage, dealer)?
        .is_some()
    {
        return Err(ContractError::AlreadyADealer);
    }

    state
        .group_addr
        .is_voting_member(&deps.querier, dealer, None)?
        .ok_or(ContractError::Unauthorized {})?;

    Ok(())
}

pub fn try_add_dealer(
    mut deps: DepsMut<'_>,
    info: MessageInfo,
    bte_key_with_proof: EncodedBTEPublicKeyWithProof,
    announce_address: String,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::PublicKeySubmission)?;
    let state = STATE.load(deps.storage)?;

    verify_dealer(deps.branch(), &state, &info.sender)?;

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
        announce_address,
        assigned_index: node_index,
    };
    dealers_storage::current_dealers().save(deps.storage, &info.sender, &dealer_details)?;

    Ok(Response::new().add_attribute("node_index", node_index.to_string()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::epoch_state::transactions::advance_epoch_state;
    use crate::support::tests::fixtures::dealer_details_fixture;
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::ADMIN_ADDRESS;
    use cosmwasm_std::testing::mock_info;

    #[test]
    fn invalid_state() {
        let mut deps = helpers::init_contract();
        let owner = Addr::unchecked("owner");
        let info = mock_info(owner.as_str(), &[]);
        let dealer_details = dealer_details_fixture(1);
        let bte_key_with_proof = String::from("bte_key_with_proof");
        let announce_address = String::from("localhost:8000");

        advance_epoch_state(deps.as_mut(), mock_info(ADMIN_ADDRESS, &[])).unwrap();

        let ret = try_add_dealer(
            deps.as_mut(),
            info.clone(),
            bte_key_with_proof.clone(),
            announce_address.clone(),
        )
        .unwrap_err();
        assert_eq!(
            ret,
            ContractError::IncorrectEpochState {
                current_state: EpochState::DealingExchange.to_string(),
                expected_state: EpochState::default().to_string(),
            }
        );
    }
}
