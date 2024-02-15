// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::epoch_state::storage::CURRENT_EPOCH;
use crate::error::ContractError;
use crate::state::storage::STATE;
use crate::verification_key_shares::storage::{dealers, verified_dealers};
use cosmwasm_std::{Addr, Deps, StdResult, Storage};
use cw4::Member;
use nym_coconut_dkg_common::types::{Epoch, EpochState};

fn all_group_members(deps: &Deps) -> Result<Vec<Member>, ContractError> {
    // the maximum limit for members queries is 30.
    // if we're ever thinking of going beyond it, we should fix it properly
    // by the DKG contract owning the group contract (i.e. init inside init)
    // and proxying all member changes and thus memoizing the members.
    // alternatively by adding hooks to the contract to inform us about any changes
    let members =
        STATE
            .load(deps.storage)?
            .group_addr
            .list_members(&deps.querier, None, Some(30))?;

    // if we're at the limit...
    if members.len() == 30 {
        return Err(ContractError::PossiblyIncompleteGroupMembersQuery);
    }

    Ok(members)
}

// check if we completed the state, so we could short circuit the deadline
pub(crate) fn check_state_completion(
    storage: &dyn Storage,
    epoch: &Epoch,
) -> Result<bool, ContractError> {
    let contract_state = STATE.load(storage)?;

    match epoch.state {
        EpochState::WaitingInitialisation => Ok(false),
        // to check this one we'd need to query for all group members, but we can't rely on this
        EpochState::PublicKeySubmission { .. } => Ok(false),

        // if every dealer has submitted all dealings, we're done
        EpochState::DealingExchange { .. } => {
            let expected_dealings =
                contract_state.key_size * epoch.state_progress.registered_dealers;
            Ok(expected_dealings == epoch.state_progress.submitted_dealings)
        }

        // if every dealer has submitted its partial key, we're done
        EpochState::VerificationKeySubmission { .. } => Ok(epoch
            .state_progress
            .submitted_key_shares
            == epoch.state_progress.registered_dealers),

        // no short-circuiting this one since the voting is happening in the multisig contract
        EpochState::VerificationKeyValidation { .. } => Ok(false),

        // if every submitted partial key has been verified, we're done
        EpochState::VerificationKeyFinalization { .. } => {
            Ok(epoch.state_progress.verified_keys == epoch.state_progress.submitted_key_shares)
        }
        EpochState::InProgress => Ok(false),
    }
}

/// Checks whether the DKG needs to undergo full reset.
/// This is determined by whether the initial set of validator changed by more than a threshold number
/// of parties joining or leaving.
pub(crate) fn needs_reset(deps: Deps) -> Result<bool, ContractError> {
    let current_state = CURRENT_EPOCH.load(deps.storage)?.state;

    // there is a theoretical edge case where we add/remove an extra member during an in-progress exchange
    // thus possibly needing reset immediately after it's done. an optimization would be to just scrap it and start it over
    // but since members are added manually, we don't have to worry about it too much for now.
    if !current_state.is_in_progress() {
        return Err(ContractError::CantResetDuringExchange);
    }

    let group_members = all_group_members(&deps)?;

    // below threshold => must reset since we can't do anything

    todo!()
}

/// Checks whether the DKG needs to undergo resharing.
/// This is determined by whether any new group members has joined the associated group contract
pub(crate) fn needs_resharing(deps: Deps) -> Result<bool, ContractError> {
    let current_state = CURRENT_EPOCH.load(deps.storage)?.state;

    // there is a theoretical edge case where we add an extra member during an in-progress exchange
    // thus needing resharing immediately after it's done. an optimization would be to just scrap it and start it over
    // but since members are added manually, we don't have to worry about it too much for now.
    if !current_state.is_in_progress() {
        return Err(ContractError::CantReshareDuringExchange);
    }

    // TODO: we need cw4 hooks here to resolve those expensive queries
    let group_members = all_group_members(&deps)?;
    let epoch_dealers = dealers(deps.storage)?;

    // if somebody has been a dealer, but hasn't been verified, tough luck
    // we only allow for resharing for new members

    // check if we have any new members
    for member in group_members {
        if !epoch_dealers.contains_key(&Addr::unchecked(member.addr)) {
            return Ok(true);
        }
    }

    // TODO: something about reset here

    Ok(false)
}

pub(crate) fn check_epoch_state(
    storage: &dyn Storage,
    against: EpochState,
) -> Result<(), ContractError> {
    let epoch_state = CURRENT_EPOCH.load(storage)?.state;
    if epoch_state != against {
        Err(ContractError::IncorrectEpochState {
            current_state: epoch_state.to_string(),
            expected_state: against.to_string(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::testing::mock_env;
    use nym_coconut_dkg_common::types::{Epoch, TimeConfiguration};

    #[test]
    pub fn check_state() {
        let mut deps = init_contract();
        let env = mock_env();

        for fixed_state in EpochState::first().all_until(EpochState::InProgress) {
            CURRENT_EPOCH
                .save(
                    deps.as_mut().storage,
                    &Epoch::new(fixed_state, 0, TimeConfiguration::default(), env.block.time),
                )
                .unwrap();
            for against_state in EpochState::first().all_until(EpochState::InProgress) {
                let ret = check_epoch_state(deps.as_mut().storage, against_state);
                if fixed_state == against_state {
                    assert!(ret.is_ok());
                } else {
                    assert!(ret.is_err());
                }
            }
        }
    }
}
