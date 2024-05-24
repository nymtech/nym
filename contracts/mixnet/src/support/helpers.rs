// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::gateways::storage as gateways_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::PendingMixNodeChanges;
use mixnet_contract_common::{EpochState, EpochStatus, IdentityKeyRef, MixNodeBond};

// helper trait to attach `Msg` to a response if it's provided
#[allow(dead_code)]
pub(crate) trait AttachOptionalMessage<T> {
    fn add_optional_message(self, msg: Option<impl Into<CosmosMsg<T>>>) -> Self;
}

impl<T> AttachOptionalMessage<T> for Response<T> {
    fn add_optional_message(self, msg: Option<impl Into<CosmosMsg<T>>>) -> Self {
        if let Some(msg) = msg {
            self.add_message(msg)
        } else {
            self
        }
    }
}

pub(crate) trait AttachSendTokens {
    fn send_tokens(self, to: impl AsRef<str>, amount: Coin) -> Self;
}

impl<T> AttachSendTokens for Response<T> {
    fn send_tokens(self, to: impl AsRef<str>, amount: Coin) -> Self {
        self.add_message(BankMsg::Send {
            to_address: to.as_ref().to_string(),
            amount: vec![amount],
        })
    }
}

// pub fn debug_with_visibility<S: Into<String>>(api: &dyn Api, msg: S) {
//     api.debug(&*format!("\n\n\n=========================================\n{}\n=========================================\n\n\n", msg.into()));
// }

pub(crate) fn validate_pledge(
    mut pledge: Vec<Coin>,
    minimum_pledge: Coin,
) -> Result<Coin, MixnetContractError> {
    // check if anything was put as bond
    if pledge.is_empty() {
        return Err(MixnetContractError::NoBondFound);
    }

    if pledge.len() > 1 {
        return Err(MixnetContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if pledge[0].denom != minimum_pledge.denom {
        return Err(MixnetContractError::WrongDenom {
            received: pledge[0].denom.clone(),
            expected: minimum_pledge.denom,
        });
    }

    // check that the pledge contains the minimum amount of tokens
    if pledge[0].amount < minimum_pledge.amount {
        return Err(MixnetContractError::InsufficientPledge {
            received: pledge[0].clone(),
            minimum: minimum_pledge,
        });
    }

    // throughout this function we've been using the value at `pledge[0]` without problems
    // (plus we have even validated that the vec is not empty), so the unwrap here is absolutely fine,
    // since it cannot possibly fail without UB
    #[allow(clippy::unwrap_used)]
    Ok(pledge.pop().unwrap())
}

pub(crate) fn validate_delegation_stake(
    mut delegation: Vec<Coin>,
    minimum_delegation: Option<Coin>,
    expected_denom: String,
) -> Result<Coin, MixnetContractError> {
    // check if anything was put as delegation
    if delegation.is_empty() {
        return Err(MixnetContractError::EmptyDelegation);
    }

    if delegation.len() > 1 {
        return Err(MixnetContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if delegation[0].denom != expected_denom {
        return Err(MixnetContractError::WrongDenom {
            received: delegation[0].denom.clone(),
            expected: expected_denom,
        });
    }

    // if we have a minimum set, check if enough tokens were sent, otherwise just check if its non-zero
    if let Some(minimum_delegation) = minimum_delegation {
        if delegation[0].amount < minimum_delegation.amount {
            return Err(MixnetContractError::InsufficientDelegation {
                received: delegation[0].clone(),
                minimum: minimum_delegation,
            });
        }
    } else if delegation[0].amount.is_zero() {
        return Err(MixnetContractError::EmptyDelegation);
    }

    // throughout this function we've been using the value at `delegation[0]` without problems
    // (plus we have even validated that the vec is not empty), so the unwrap here is absolutely fine,
    // since it cannot possibly fail without UB
    #[allow(clippy::unwrap_used)]
    Ok(delegation.pop().unwrap())
}

pub(crate) fn ensure_epoch_in_progress_state(
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    let epoch_status = crate::interval::storage::current_epoch_status(storage)?;
    if !matches!(epoch_status.state, EpochState::InProgress) {
        return Err(MixnetContractError::EpochAdvancementInProgress {
            current_state: epoch_status.state,
        });
    }
    Ok(())
}

// pub(crate) fn ensure_mix_rewarding_state(storage: &dyn Storage) -> Result<(), MixnetContractError> {
//     let epoch_status = crate::interval::storage::current_epoch_status(storage)?;
//     if !matches!(epoch_status.state, EpochState::Rewarding { .. }) {
//         return Err(MixnetContractError::EpochNotInMixRewardingState {
//             current_state: epoch_status.state,
//         });
//     }
//     Ok(())
// }
//
// pub(crate) fn ensure_event_reconciliation_state(
//     storage: &dyn Storage,
// ) -> Result<(), MixnetContractError> {
//     let epoch_status = crate::interval::storage::current_epoch_status(storage)?;
//     if !matches!(epoch_status.state, EpochState::ReconcilingEvents) {
//         return Err(MixnetContractError::EpochNotInEventReconciliationState {
//             current_state: epoch_status.state,
//         });
//     }
//     Ok(())
// }
//
// pub(crate) fn ensure_epoch_advancement_state(
//     storage: &dyn Storage,
// ) -> Result<(), MixnetContractError> {
//     let epoch_status = crate::interval::storage::current_epoch_status(storage)?;
//     if !matches!(epoch_status.state, EpochState::AdvancingEpoch) {
//         return Err(MixnetContractError::EpochNotInAdvancementState {
//             current_state: epoch_status.state,
//         });
//     }
//     Ok(())
// }

pub(crate) fn ensure_is_authorized(
    sender: &Addr,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    if sender != crate::mixnet_contract_settings::storage::rewarding_validator_address(storage)? {
        return Err(MixnetContractError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn ensure_can_advance_epoch(
    sender: &Addr,
    storage: &dyn Storage,
) -> Result<EpochStatus, MixnetContractError> {
    let epoch_status = crate::interval::storage::current_epoch_status(storage)?;
    if sender != epoch_status.being_advanced_by {
        // well, we know we're going to throw an error now,
        // but we might as well also check if we're even a validator
        // to return a possibly better error message
        ensure_is_authorized(sender, storage)?;
        return Err(MixnetContractError::RewardingValidatorMismatch {
            current_validator: sender.clone(),
            chosen_validator: epoch_status.being_advanced_by,
        });
    }
    Ok(epoch_status)
}

pub(crate) fn ensure_is_owner(
    sender: Addr,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    if sender
        != crate::mixnet_contract_settings::storage::CONTRACT_STATE
            .load(storage)?
            .owner
    {
        return Err(MixnetContractError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn ensure_bonded(bond: &MixNodeBond) -> Result<(), MixnetContractError> {
    if bond.is_unbonding {
        return Err(MixnetContractError::MixnodeIsUnbonding {
            mix_id: bond.mix_id,
        });
    }
    Ok(())
}

pub(crate) fn ensure_no_pending_pledge_changes(
    pending_changes: &PendingMixNodeChanges,
) -> Result<(), MixnetContractError> {
    if let Some(pending_event_id) = pending_changes.pledge_change {
        return Err(MixnetContractError::PendingPledgeChange { pending_event_id });
    }
    Ok(())
}

// check if the target address has already bonded a mixnode or gateway,
// in either case, return an appropriate error
pub(crate) fn ensure_no_existing_bond(
    sender: &Addr,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    if mixnodes_storage::mixnode_bonds()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(MixnetContractError::AlreadyOwnsMixnode);
    }

    if gateways_storage::gateways()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(MixnetContractError::AlreadyOwnsGateway);
    }

    Ok(())
}

pub(crate) fn decode_ed25519_identity_key(
    encoded: IdentityKeyRef,
) -> Result<[u8; 32], MixnetContractError> {
    let mut public_key = [0u8; 32];
    let used = bs58::decode(encoded)
        .into(&mut public_key)
        .map_err(|err| MixnetContractError::MalformedEd25519IdentityKey(err.to_string()))?;

    if used != 32 {
        return Err(MixnetContractError::MalformedEd25519IdentityKey(
            "Too few bytes provided for the public key".into(),
        ));
    }

    Ok(public_key)
}
