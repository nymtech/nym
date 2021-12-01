// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::error::ContractError;
use crate::mixnodes::storage as mixnodes_storage;
use cosmwasm_std::{Addr, Storage, Uint128};
use mixnet_contract::mixnode::DelegatorRewardParams;
use mixnet_contract::{
    IdentityKey, IdentityKeyRef, PendingDelegatorRewarding, RewardingResult, RewardingStatus,
};

pub(crate) fn update_post_rewarding_storage(
    storage: &mut dyn Storage,
    mix_identity: IdentityKeyRef,
    operator_reward: Uint128,
    delegators_reward: Uint128,
) -> Result<(), ContractError> {
    if operator_reward == Uint128::zero() && delegators_reward == Uint128::zero() {
        return Ok(());
    }

    // update bond
    if operator_reward > Uint128::zero() {
        mixnodes_storage::mixnodes().update(storage, mix_identity, |current_bond| {
            match current_bond {
                None => Err(ContractError::MixNodeBondNotFound {
                    identity: mix_identity.to_string(),
                }),
                Some(mut mixnode_bond) => {
                    mixnode_bond.bond_amount.amount += operator_reward;
                    Ok(mixnode_bond)
                }
            }
        })?;
    }

    // update total_delegation
    if delegators_reward > Uint128::zero() {
        mixnodes_storage::TOTAL_DELEGATION.update(storage, mix_identity, |current_total| {
            match current_total {
                None => Err(ContractError::MixNodeBondNotFound {
                    identity: mix_identity.to_string(),
                }),
                Some(current_total) => Ok(current_total + delegators_reward),
            }
        })?;
    }

    // update reward pool
    storage::decr_reward_pool(storage, operator_reward + delegators_reward)?;

    Ok(())
}

pub(crate) fn update_rewarding_status(
    storage: &mut dyn Storage,
    rewarding_interval_nonce: u32,
    mix_identity: IdentityKey,
    rewarding_results: RewardingResult,
    next_start: Option<Addr>,
    delegators_rewarding_params: DelegatorRewardParams,
) -> Result<(), ContractError> {
    if let Some(next_start) = next_start {
        storage::REWARDING_STATUS.save(
            storage,
            (rewarding_interval_nonce.into(), mix_identity),
            &RewardingStatus::PendingNextDelegatorPage(PendingDelegatorRewarding {
                running_results: rewarding_results,
                next_start,
                rewarding_params: delegators_rewarding_params,
            }),
        )?;
    } else {
        storage::REWARDING_STATUS.save(
            storage,
            (rewarding_interval_nonce.into(), mix_identity),
            &RewardingStatus::Complete(rewarding_results),
        )?;
    }

    Ok(())
}
