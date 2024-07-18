// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::gateways::storage as gateways_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{wasm_execute, Addr, BankMsg, Coin, CosmosMsg, MessageInfo, Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::PendingMixNodeChanges;
use mixnet_contract_common::{EpochState, EpochStatus, IdentityKeyRef, MixId, MixNodeBond};
use nym_contracts_common::Percent;
use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;

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

// another helper trait to remove some duplicate code and consolidate comments regarding
// possible epoch progression halting behaviour
pub(crate) trait VestingTracking
where
    Self: Sized,
{
    fn maybe_add_track_vesting_undelegation_message(
        self,
        storage: &dyn Storage,
        proxy: Option<Addr>,
        owner: String,
        mix_id: MixId,
        amount: Coin,
    ) -> Result<Self, MixnetContractError>;

    fn maybe_add_track_vesting_unbond_mixnode_message(
        self,
        storage: &dyn Storage,
        proxy: Option<Addr>,
        owner: String,
        amount: Coin,
    ) -> Result<Self, MixnetContractError>;

    fn maybe_add_track_vesting_decrease_mixnode_pledge(
        self,
        storage: &dyn Storage,
        proxy: Option<Addr>,
        owner: String,
        amount: Coin,
    ) -> Result<Self, MixnetContractError>;
}

impl VestingTracking for Response {
    fn maybe_add_track_vesting_undelegation_message(
        self,
        storage: &dyn Storage,
        proxy: Option<Addr>,
        owner: String,
        mix_id: MixId,
        amount: Coin,
    ) -> Result<Self, MixnetContractError> {
        // if there's a proxy set (i.e. the vesting contract), send the track message
        if let Some(proxy) = proxy {
            let vesting_contract = mixnet_params_storage::vesting_contract_address(storage)?;

            // Note: this can INTENTIONALLY cause epoch progression halt if the proxy is not the vesting contract
            // But this is fine,  since this situation should have NEVER occurred in the first place
            // (as all 'on_behalf' methods, including 'DelegateToMixnodeOnBehalf' that got us here,
            // explicitly require the proxy to be the vesting contract)
            // 'fixing' it would require manually inspecting the problematic event, investigating
            // it's cause and manually (presumably via migration) clearing it.
            if proxy != vesting_contract {
                return Err(MixnetContractError::ProxyIsNotVestingContract {
                    received: proxy,
                    vesting_contract,
                });
            }

            let msg = VestingContractExecuteMsg::TrackUndelegation {
                owner,
                mix_id,
                amount,
            };

            let track_undelegate_message = wasm_execute(proxy, &msg, vec![])?;
            Ok(self.add_message(track_undelegate_message))
        } else {
            // there's no proxy so nothing to do
            Ok(self)
        }
    }

    fn maybe_add_track_vesting_unbond_mixnode_message(
        self,
        storage: &dyn Storage,
        proxy: Option<Addr>,
        owner: String,
        amount: Coin,
    ) -> Result<Self, MixnetContractError> {
        // if there's a proxy set (i.e. the vesting contract), send the track message
        if let Some(proxy) = proxy {
            let vesting_contract = mixnet_params_storage::vesting_contract_address(storage)?;

            // exactly the same possible halting behaviour as in `maybe_add_track_vesting_undelegation_message`.
            if proxy != vesting_contract {
                return Err(MixnetContractError::ProxyIsNotVestingContract {
                    received: proxy,
                    vesting_contract,
                });
            }

            let msg = VestingContractExecuteMsg::TrackUnbondMixnode { owner, amount };
            let track_unbond_message = wasm_execute(proxy, &msg, vec![])?;
            Ok(self.add_message(track_unbond_message))
        } else {
            // there's no proxy so nothing to do
            Ok(self)
        }
    }

    fn maybe_add_track_vesting_decrease_mixnode_pledge(
        self,
        storage: &dyn Storage,
        proxy: Option<Addr>,
        owner: String,
        amount: Coin,
    ) -> Result<Self, MixnetContractError> {
        if let Some(proxy) = proxy {
            let vesting_contract = mixnet_params_storage::vesting_contract_address(storage)?;

            // exactly the same possible halting behaviour as in `maybe_add_track_vesting_undelegation_message`.
            if proxy != vesting_contract {
                return Err(MixnetContractError::ProxyIsNotVestingContract {
                    received: proxy,
                    vesting_contract,
                });
            }

            let msg = VestingContractExecuteMsg::TrackDecreasePledge { owner, amount };
            let track_decrease_pledge_message = wasm_execute(proxy, &msg, vec![])?;
            Ok(self.add_message(track_decrease_pledge_message))
        } else {
            // there's no proxy so nothing to do
            Ok(self)
        }
    }
}

// pub fn debug_with_visibility<S: Into<String>>(api: &dyn Api, msg: S) {
//     api.debug(&*format!("\n\n\n=========================================\n{}\n=========================================\n\n\n", msg.into()));
// }

/// Attempts to construct a `BankMsg` to send specified tokens to the provided
/// proxy address. If that's unavailable, the `BankMsg` will use the "owner" as the
/// "to_address".
pub(crate) fn send_to_proxy_or_owner(
    proxy: &Option<Addr>,
    owner: &Addr,
    amount: Vec<Coin>,
) -> BankMsg {
    BankMsg::Send {
        to_address: proxy.as_ref().unwrap_or(owner).to_string(),
        amount,
    }
}

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

pub(crate) fn ensure_proxy_match(
    actual: &Option<Addr>,
    expected: &Option<Addr>,
) -> Result<(), MixnetContractError> {
    if actual != expected {
        return Err(MixnetContractError::ProxyMismatch {
            existing: expected
                .as_ref()
                .map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
            incoming: actual
                .as_ref()
                .map_or_else(|| "None".to_string(), |a| a.as_str().to_string()),
        });
    }
    Ok(())
}

pub(crate) fn ensure_sent_by_vesting_contract(
    info: &MessageInfo,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    let vesting_contract_address =
        crate::mixnet_contract_settings::storage::vesting_contract_address(storage)?;
    if info.sender != vesting_contract_address {
        Err(MixnetContractError::SenderIsNotVestingContract {
            received: info.sender.clone(),
            vesting_contract: vesting_contract_address,
        })
    } else {
        Ok(())
    }
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

pub(crate) fn ensure_profit_margin_within_range(
    storage: &dyn Storage,
    profit_margin: Percent,
) -> Result<(), MixnetContractError> {
    let range = mixnet_params_storage::profit_margin_range(storage)?;
    if !range.within_range(profit_margin) {
        return Err(MixnetContractError::ProfitMarginOutsideRange {
            provided: profit_margin,
            range,
        });
    }

    Ok(())
}
