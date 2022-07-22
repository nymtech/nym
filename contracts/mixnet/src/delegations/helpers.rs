// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Coin, Decimal, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::MixNodeRewarding;
use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;
use mixnet_contract_common::Delegation;

pub(crate) fn undelegate(
    store: &mut dyn Storage,
    delegation: Delegation,
    mut mix_rewarding: MixNodeRewarding,
) -> Result<Coin, MixnetContractError> {
    let reward = mix_rewarding.determine_delegation_reward(&delegation);
    mix_rewarding.remove_full_delegation_amount(delegation.dec_amount() + reward)?;

    let truncated_reward = truncate_reward_amount(reward);
    let mut amount = delegation.amount.clone();
    amount.amount += truncated_reward;

    rewards_storage::MIXNODE_REWARDING.save(store, delegation.node_id, &mix_rewarding)?;
    storage::delegations().replace(store, delegation.storage_key(), None, Some(&delegation))?;

    Ok(amount)
}
