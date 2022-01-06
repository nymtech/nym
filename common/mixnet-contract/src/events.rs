// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::NodeRewardResult;
use crate::{ContractStateParams, Delegation, IdentityKeyRef, Layer};
use cosmwasm_std::{Addr, Coin, Event, Uint128};

// event types
pub const REWARDING_EVENT_TYPE: &str = "rewarding";
pub const DELEGATION_EVENT_TYPE: &str = "delegation";
pub const UNDELEGATION_EVENT_TYPE: &str = "undelegation";
pub const GATEWAY_BONDING_EVENT_TYPE: &str = "gateway_bonding";
pub const GATEWAY_UNBONDING_EVENT_TYPE: &str = "gateway_unbonding";
pub const MIXNODE_BONDING_EVENT_TYPE: &str = "mixnode_bonding";
pub const MIXNODE_UNBONDING_EVENT_TYPE: &str = "mixnode_unbonding";
pub const SETTINGS_UPDATE_EVENT_TYPE: &str = "settings_update";
pub const BEGIN_REWARDING_EVENT_TYPE: &str = "begin_rewarding";
pub const OPERATOR_REWARDING_EVENT_TYPE: &str = "mix_rewarding";
pub const MIX_DELEGATORS_REWARDING_EVENT_TYPE: &str = "mix_delegators_rewarding";
pub const FINISH_REWARDING_EVENT_TYPE: &str = "finish_rewarding";

// attributes that are used in multiple places
pub const OWNER_KEY: &str = "owner";
pub const AMOUNT_KEY: &str = "amount";
pub const PROXY_KEY: &str = "proxy";

// event-specific attributes

// delegation/undelegation
pub const DELEGATOR_KEY: &str = "delegator";
pub const DELEGATION_TARGET_KEY: &str = "delegation_target";
pub const DELEGATION_HEIGHT_KEY: &str = "delegation_latest_block_height";

// bonding/unbonding
pub const NODE_IDENTITY_KEY: &str = "identity";
pub const ASSIGNED_LAYER_KEY: &str = "assigned_layer";

// settings change
pub const OLD_MINIMUM_MIXNODE_PLEDGE_KEY: &str = "old_minimum_mixnode_pledge";
pub const OLD_MINIMUM_GATEWAY_PLEDGE_KEY: &str = "old_minimum_gateway_pledge";
pub const OLD_MIXNODE_REWARDED_SET_SIZE_KEY: &str = "old_mixnode_rewarded_set_size";
pub const OLD_MIXNODE_ACTIVE_SET_SIZE_KEY: &str = "old_mixnode_active_set_size";
pub const OLD_ACTIVE_SET_WORK_FACTOR_KEY: &str = "old_active_set_work_factor";

pub const NEW_MINIMUM_MIXNODE_PLEDGE_KEY: &str = "new_minimum_mixnode_pledge";
pub const NEW_MINIMUM_GATEWAY_PLEDGE_KEY: &str = "new_minimum_gateway_pledge";
pub const NEW_MIXNODE_REWARDED_SET_SIZE_KEY: &str = "new_mixnode_rewarded_set_size";
pub const NEW_MIXNODE_ACTIVE_SET_SIZE_KEY: &str = "new_mixnode_active_set_size";
pub const NEW_ACTIVE_SET_WORK_FACTOR_KEY: &str = "new_active_set_work_factor";

// rewarding
pub const REWARDING_INTERVAL_NONCE_KEY: &str = "rewarding_interval_nonce";
pub const TOTAL_MIXNODE_REWARD_KEY: &str = "total_node_reward";
pub const OPERATOR_REWARD_KEY: &str = "operator_reward";
pub const LAMBDA_KEY: &str = "lambda";
pub const SIGMA_KEY: &str = "sigma";
pub const DISTRIBUTED_DELEGATION_REWARDS_KEY: &str = "distributed_delegation_rewards";
pub const FURTHER_DELEGATIONS_TO_REWARD_KEY: &str = "further_delegations";
pub const NO_REWARD_REASON_KEY: &str = "no_reward_reason";
pub const BOND_NOT_FOUND_VALUE: &str = "bond_not_found";
pub const BOND_TOO_FRESH_VALUE: &str = "bond_too_fresh";
pub const ZERO_UPTIME_VALUE: &str = "zero_uptime";

pub fn new_delegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    mix_identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(DELEGATION_EVENT_TYPE).add_attribute(DELEGATOR_KEY, delegator);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_identity)
}

pub fn new_undelegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    old_delegation: &Delegation,
    mix_identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(UNDELEGATION_EVENT_TYPE).add_attribute(DELEGATOR_KEY, delegator);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(AMOUNT_KEY, old_delegation.amount.to_string())
        .add_attribute(
            DELEGATION_HEIGHT_KEY,
            old_delegation.block_height.to_string(),
        )
        .add_attribute(DELEGATION_TARGET_KEY, mix_identity)
}

pub fn new_gateway_bonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(GATEWAY_BONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_gateway_unbonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(GATEWAY_UNBONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_mixnode_bonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
    assigned_layer: Layer,
) -> Event {
    let mut event = Event::new(MIXNODE_BONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(ASSIGNED_LAYER_KEY, assigned_layer)
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_mixnode_unbonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef,
) -> Event {
    let mut event = Event::new(MIXNODE_UNBONDING_EVENT_TYPE)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_settings_update_event(
    old_params: &ContractStateParams,
    new_params: &ContractStateParams,
) -> Event {
    let mut event = Event::new(SETTINGS_UPDATE_EVENT_TYPE);

    if old_params.minimum_mixnode_pledge != new_params.minimum_mixnode_pledge {
        event = event
            .add_attribute(
                OLD_MINIMUM_MIXNODE_PLEDGE_KEY,
                old_params.minimum_mixnode_pledge,
            )
            .add_attribute(
                NEW_MINIMUM_MIXNODE_PLEDGE_KEY,
                new_params.minimum_mixnode_pledge,
            )
    }

    if old_params.minimum_gateway_pledge != new_params.minimum_gateway_pledge {
        event = event
            .add_attribute(
                OLD_MINIMUM_GATEWAY_PLEDGE_KEY,
                old_params.minimum_gateway_pledge,
            )
            .add_attribute(
                NEW_MINIMUM_GATEWAY_PLEDGE_KEY,
                new_params.minimum_gateway_pledge,
            )
    }

    if old_params.mixnode_rewarded_set_size != new_params.mixnode_rewarded_set_size {
        event = event
            .add_attribute(
                OLD_MIXNODE_REWARDED_SET_SIZE_KEY,
                old_params.mixnode_rewarded_set_size.to_string(),
            )
            .add_attribute(
                NEW_MIXNODE_REWARDED_SET_SIZE_KEY,
                new_params.mixnode_rewarded_set_size.to_string(),
            )
    }

    if old_params.mixnode_active_set_size != new_params.mixnode_active_set_size {
        event = event
            .add_attribute(
                OLD_MIXNODE_ACTIVE_SET_SIZE_KEY,
                old_params.mixnode_active_set_size.to_string(),
            )
            .add_attribute(
                NEW_MIXNODE_ACTIVE_SET_SIZE_KEY,
                new_params.mixnode_active_set_size.to_string(),
            )
    }

    if old_params.active_set_work_factor != new_params.active_set_work_factor {
        event = event
            .add_attribute(
                OLD_ACTIVE_SET_WORK_FACTOR_KEY,
                old_params.active_set_work_factor.to_string(),
            )
            .add_attribute(
                NEW_ACTIVE_SET_WORK_FACTOR_KEY,
                new_params.active_set_work_factor.to_string(),
            )
    }

    event
}

pub fn new_begin_rewarding_event(rewarding_interval_nonce: u32) -> Event {
    Event::new(BEGIN_REWARDING_EVENT_TYPE).add_attribute(
        REWARDING_INTERVAL_NONCE_KEY,
        rewarding_interval_nonce.to_string(),
    )
}

pub fn new_finish_rewarding_event(rewarding_interval_nonce: u32) -> Event {
    Event::new(FINISH_REWARDING_EVENT_TYPE).add_attribute(
        REWARDING_INTERVAL_NONCE_KEY,
        rewarding_interval_nonce.to_string(),
    )
}

pub fn new_not_found_mix_operator_rewarding_event(
    rewarding_interval_nonce: u32,
    identity: IdentityKeyRef,
) -> Event {
    Event::new(OPERATOR_REWARDING_EVENT_TYPE)
        .add_attribute(
            REWARDING_INTERVAL_NONCE_KEY,
            rewarding_interval_nonce.to_string(),
        )
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(NO_REWARD_REASON_KEY, BOND_NOT_FOUND_VALUE)
}

pub fn new_too_fresh_bond_mix_operator_rewarding_event(
    rewarding_interval_nonce: u32,
    identity: IdentityKeyRef,
) -> Event {
    Event::new(OPERATOR_REWARDING_EVENT_TYPE)
        .add_attribute(
            REWARDING_INTERVAL_NONCE_KEY,
            rewarding_interval_nonce.to_string(),
        )
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(NO_REWARD_REASON_KEY, BOND_TOO_FRESH_VALUE)
}

pub fn new_zero_uptime_mix_operator_rewarding_event(
    rewarding_interval_nonce: u32,
    identity: IdentityKeyRef,
) -> Event {
    Event::new(OPERATOR_REWARDING_EVENT_TYPE)
        .add_attribute(
            REWARDING_INTERVAL_NONCE_KEY,
            rewarding_interval_nonce.to_string(),
        )
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(NO_REWARD_REASON_KEY, ZERO_UPTIME_VALUE)
}

pub fn new_mix_operator_rewarding_event(
    rewarding_interval_nonce: u32,
    identity: IdentityKeyRef,
    node_reward_result: NodeRewardResult,
    operator_reward: Uint128,
    delegation_rewards_distributed: Uint128,
    further_delegations: bool,
) -> Event {
    Event::new(OPERATOR_REWARDING_EVENT_TYPE)
        .add_attribute(
            REWARDING_INTERVAL_NONCE_KEY,
            rewarding_interval_nonce.to_string(),
        )
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(
            TOTAL_MIXNODE_REWARD_KEY,
            node_reward_result.reward().to_string(),
        )
        .add_attribute(LAMBDA_KEY, node_reward_result.lambda().to_string())
        .add_attribute(SIGMA_KEY, node_reward_result.sigma().to_string())
        .add_attribute(OPERATOR_REWARD_KEY, operator_reward)
        .add_attribute(
            DISTRIBUTED_DELEGATION_REWARDS_KEY,
            delegation_rewards_distributed,
        )
        .add_attribute(
            FURTHER_DELEGATIONS_TO_REWARD_KEY,
            further_delegations.to_string(),
        )
}

pub fn new_mix_delegators_rewarding_event(
    rewarding_interval_nonce: u32,
    identity: IdentityKeyRef,
    delegation_rewards_distributed: Uint128,
    further_delegations: bool,
) -> Event {
    Event::new(MIX_DELEGATORS_REWARDING_EVENT_TYPE)
        .add_attribute(
            REWARDING_INTERVAL_NONCE_KEY,
            rewarding_interval_nonce.to_string(),
        )
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(
            DISTRIBUTED_DELEGATION_REWARDS_KEY,
            delegation_rewards_distributed,
        )
        .add_attribute(
            FURTHER_DELEGATIONS_TO_REWARD_KEY,
            further_delegations.to_string(),
        )
}
