// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::gateways::storage as gateways_storage;
use crate::gateways::storage::PREASSIGNED_LEGACY_IDS;
use crate::interval::storage as interval_storage;
use crate::mixnodes::storage as mixnodes_storage;
use crate::nodes::storage::next_nymnode_id_counter;
use crate::rewards::storage as rewards_storage;
use cosmwasm_std::{Addr, Coin, Env, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{
    Gateway, GatewayBond, MixNode, MixNodeBond, NodeCostParams, NodeId, NodeRewarding,
};
use nym_contracts_common::IdentityKey;

pub(crate) fn save_new_mixnode(
    storage: &mut dyn Storage,
    env: Env,
    mixnode: MixNode,
    cost_params: NodeCostParams,
    owner: Addr,
    pledge: Coin,
) -> Result<NodeId, MixnetContractError> {
    let mix_id = next_nymnode_id_counter(storage)?;
    let current_epoch = interval_storage::current_interval(storage)?.current_epoch_absolute_id();

    let mixnode_rewarding = NodeRewarding::initialise_new(cost_params, &pledge, current_epoch)?;
    let mixnode_bond = MixNodeBond {
        mix_id,
        owner,
        original_pledge: pledge,
        mix_node: mixnode,
        proxy: None,
        bonding_height: env.block.height,
        is_unbonding: false,
    };

    // save mixnode bond data
    // note that this implicitly checks for uniqueness on identity key, sphinx key and owner
    mixnodes_storage::mixnode_bonds().save(storage, mix_id, &mixnode_bond)?;

    // save rewarding data
    rewards_storage::MIXNODE_REWARDING.save(storage, mix_id, &mixnode_rewarding)?;

    Ok(mix_id)
}

pub(crate) fn save_new_gateway(
    storage: &mut dyn Storage,
    env: Env,
    gateway: Gateway,
    owner: Addr,
    pledge: Coin,
) -> Result<(IdentityKey, NodeId), MixnetContractError> {
    let gateway_identity = gateway.identity_key.clone();
    let bond = GatewayBond::new(pledge.clone(), owner.clone(), env.block.height, gateway);

    gateways_storage::gateways().save(storage, bond.identity(), &bond)?;

    let id = next_nymnode_id_counter(storage)?;
    PREASSIGNED_LEGACY_IDS.save(storage, gateway_identity.clone(), &id)?;

    Ok((gateway_identity, id))
}
