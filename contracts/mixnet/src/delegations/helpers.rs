// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Coin, Decimal, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::NodeId;

pub(crate) fn add_delegation(
    storage: &mut dyn Storage,
    amount: Coin,
    mix_id: NodeId,
) -> Result<Decimal, MixnetContractError> {
    let mut mix_rewarding = match rewards_storage::MIXNODE_REWARDING.may_load(storage, mix_id)? {
        Some(mix_rewarding) if mix_rewarding.still_bonded() => mix_rewarding,
        _ => {
            return Err(MixnetContractError::MixNodeBondNotFound { id: mix_id });
        }
    };

    let cumulative_reward_ratio = mix_rewarding.total_unit_reward;
    mix_rewarding.add_base_delegation(&amount);
    todo!()
}
