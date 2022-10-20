// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::MINIMUM_DEPOSIT;
use crate::dealers::storage as dealers_storage;
use crate::epoch_state::utils::check_epoch_state;
use crate::{ContractError, State, STATE};
use coconut_dkg_common::types::{DealerDetails, EncodedBTEPublicKeyWithProof, EpochState};
use cosmwasm_std::{Addr, Coin, DepsMut, MessageInfo, Response};

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

fn validate_dealer_deposit(state: &State, mut deposit: Vec<Coin>) -> Result<Coin, ContractError> {
    // check if anything was put as deposit
    if deposit.is_empty() {
        return Err(ContractError::NoDepositFound {
            denom: state.mix_denom.clone(),
        });
    }

    if deposit.len() > 1 {
        return Err(ContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if deposit[0].denom != state.mix_denom {
        return Err(ContractError::WrongDenom {
            denom: state.mix_denom.clone(),
        });
    }

    // check that we have at least MINIMUM_DEPOSIT coins in our deposit
    if deposit[0].amount < MINIMUM_DEPOSIT {
        return Err(ContractError::InsufficientDeposit {
            received: deposit[0].amount.into(),
            minimum: MINIMUM_DEPOSIT.into(),
        });
    }

    // the unwrap would have been safe here under all circumstances, since we checked whether the vector is empty
    // but in case something did change, change option into an error
    deposit.pop().ok_or(ContractError::NoDepositFound {
        denom: state.mix_denom.clone(),
    })
}

pub fn try_add_dealer(
    mut deps: DepsMut<'_>,
    info: MessageInfo,
    bte_key_with_proof: EncodedBTEPublicKeyWithProof,
) -> Result<Response, ContractError> {
    check_epoch_state(deps.storage, EpochState::PublicKeySubmission)?;
    let state = STATE.load(deps.storage)?;

    verify_dealer(deps.branch(), &state, &info.sender)?;

    // validate and extract sent deposit
    let deposit = validate_dealer_deposit(&state, info.funds)?;

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
        assigned_index: node_index,
        deposit,
    };
    dealers_storage::current_dealers().save(deps.storage, &info.sender, &dealer_details)?;

    Ok(Response::new().add_attribute("node_index", node_index.to_string()))
}
