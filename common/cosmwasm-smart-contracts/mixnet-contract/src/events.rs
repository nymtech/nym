// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use crate::rewarding::RewardDistribution;
use crate::{ContractStateParams, IdentityKeyRef, Interval, Layer, NodeId};
pub use contracts_common::events::*;
use cosmwasm_std::{Addr, Coin, Event, Uint128};

pub enum MixnetEventType {
    MixnodeBonding,
    PendingMixnodeUnbonding,
    MixnodeUnbonding,
    MixnodeConfigUpdate,
    PendingMixnodeCostParamsUpdate,
    MixnodeCostParamsUpdate,
    MixnodeRewarding,
}

impl From<MixnetEventType> for String {
    fn from(typ: MixnetEventType) -> Self {
        typ.to_string()
    }
}

impl ToString for MixnetEventType {
    fn to_string(&self) -> String {
        match self {
            MixnetEventType::MixnodeBonding => "mixnode_bonding",
            MixnetEventType::PendingMixnodeUnbonding => "pending_mixnode_unbonding",
            MixnetEventType::MixnodeConfigUpdate => "mixnode_config_update",
            MixnetEventType::MixnodeUnbonding => "mixnode_unbonding",
            MixnetEventType::PendingMixnodeCostParamsUpdate => "pending_mixnode_cost_params_update",
            MixnetEventType::MixnodeCostParamsUpdate => "mixnode_cost_params_update",
            MixnetEventType::MixnodeRewarding => "mix_rewarding",
        }
        .into()
    }
}

// FIXME: This should become an Enum
// event types
pub const DELEGATION_EVENT_TYPE: &str = "delegation";
pub const PENDING_DELEGATION_EVENT_TYPE: &str = "pending_delegation";
pub const UNDELEGATION_EVENT_TYPE: &str = "undelegation";
pub const PENDING_UNDELEGATION_EVENT_TYPE: &str = "pending_undelegation";
pub const GATEWAY_BONDING_EVENT_TYPE: &str = "gateway_bonding";
pub const GATEWAY_UNBONDING_EVENT_TYPE: &str = "gateway_unbonding";
pub const REWARDING_VALIDATOR_UPDATE_EVENT_TYPE: &str = "rewarding_validator_address_update";
pub const SETTINGS_UPDATE_EVENT_TYPE: &str = "settings_update";
pub const MIX_DELEGATORS_REWARDING_EVENT_TYPE: &str = "mix_delegators_rewarding";
pub const CHANGE_REWARDED_SET_EVENT_TYPE: &str = "change_rewarded_set";
pub const ADVANCE_EPOCH_EVENT_TYPE: &str = "advance_epoch";
pub const COMPOUND_DELEGATOR_REWARD_EVENT_TYPE: &str = "compound_delegator_reward";
pub const CLAIM_DELEGATOR_REWARD_EVENT_TYPE: &str = "claim_delegator_reward";
pub const COMPOUND_OPERATOR_REWARD_EVENT_TYPE: &str = "compound_operator_reward";
pub const CLAIM_OPERATOR_REWARD_EVENT_TYPE: &str = "claim_operator_reward";

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
pub const NODE_ID_KEY: &str = "node_id";
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

pub const OLD_REWARDING_VALIDATOR_ADDRESS_KEY: &str = "old_rewarding_validator_address";
pub const NEW_REWARDING_VALIDATOR_ADDRESS_KEY: &str = "new_rewarding_validator_address";

pub const UPDATED_MIXNODE_CONFIG_KEY: &str = "updated_mixnode_config";
pub const UPDATED_MIXNODE_COST_PARAMS_KEY: &str = "updated_mixnode_cost_params";

// rewarding
pub const INTERVAL_KEY: &str = "interval_details";
pub const TOTAL_MIXNODE_REWARD_KEY: &str = "total_node_reward";
pub const TOTAL_PLEDGE_KEY: &str = "pledge";
pub const TOTAL_DELEGATIONS_KEY: &str = "delegated";
pub const OPERATOR_REWARD_KEY: &str = "operator_reward";
pub const DELEGATES_REWARD_KEY: &str = "delegates_reward";

pub const DISTRIBUTED_DELEGATION_REWARDS_KEY: &str = "distributed_delegation_rewards";
pub const FURTHER_DELEGATIONS_TO_REWARD_KEY: &str = "further_delegations";
pub const NO_REWARD_REASON_KEY: &str = "no_reward_reason";
pub const BOND_NOT_FOUND_VALUE: &str = "bond_not_found";
pub const BOND_TOO_FRESH_VALUE: &str = "bond_too_fresh";
pub const ZERO_PERFORMANCE_VALUE: &str = "zero_performance";

// rewarded set update
pub const ACTIVE_SET_SIZE_KEY: &str = "active_set_size";
pub const REWARDED_SET_SIZE_KEY: &str = "rewarded_set_size";
pub const NODES_IN_REWARDED_SET_KEY: &str = "nodes_in_rewarded_set";
pub const CURRENT_INTERVAL_ID_KEY: &str = "current_interval";

pub const NEW_CURRENT_INTERVAL_KEY: &str = "new_current_interval";
pub const NEW_CURRENT_EPOCH_KEY: &str = "new_current_epoch";
pub const BLOCK_HEIGHT_KEY: &str = "block_height";
pub const RECONCILIATION_ERROR_EVENT: &str = "reconciliation_error";

pub fn new_error_event(err: String) -> Event {
    Event::new(RECONCILIATION_ERROR_EVENT).add_attribute("error", err)
}

pub fn new_delegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    mix_identity: IdentityKeyRef<'_>,
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

pub fn new_pending_delegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    mix_id: NodeId,
) -> Event {
    Event::new(PENDING_DELEGATION_EVENT_TYPE)
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_optional_argument(PROXY_KEY, proxy.as_ref())
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_id.to_string())
}

pub fn new_compound_operator_reward_event(owner: &Addr, amount: Uint128) -> Event {
    let event = Event::new(COMPOUND_OPERATOR_REWARD_EVENT_TYPE).add_attribute(OWNER_KEY, owner);
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_claim_operator_reward_event(owner: &Addr, amount: Uint128) -> Event {
    let event = Event::new(CLAIM_OPERATOR_REWARD_EVENT_TYPE).add_attribute(OWNER_KEY, owner);
    event.add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_compound_delegator_reward_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    amount: Uint128,
    mix_identity: IdentityKeyRef<'_>,
) -> Event {
    let mut event =
        Event::new(COMPOUND_DELEGATOR_REWARD_EVENT_TYPE).add_attribute(DELEGATOR_KEY, delegator);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_identity)
        .add_attribute(DELEGATOR_KEY, delegator)
}

pub fn new_claim_delegator_reward_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    amount: Uint128,
    mix_identity: IdentityKeyRef<'_>,
) -> Event {
    let mut event =
        Event::new(CLAIM_DELEGATOR_REWARD_EVENT_TYPE).add_attribute(DELEGATOR_KEY, delegator);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_identity)
        .add_attribute(DELEGATOR_KEY, delegator)
}

pub fn new_undelegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    mix_identity: IdentityKeyRef<'_>,
    amount: Uint128,
) -> Event {
    let mut event = Event::new(UNDELEGATION_EVENT_TYPE).add_attribute(DELEGATOR_KEY, delegator);

    if let Some(proxy) = proxy {
        event = event.add_attribute(PROXY_KEY, proxy)
    }

    // coin implements Display trait and we use that implementation here
    event
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_identity)
}

pub fn new_pending_undelegation_event(
    delegator: &Addr,
    proxy: &Option<Addr>,
    mix_id: NodeId,
) -> Event {
    Event::new(PENDING_UNDELEGATION_EVENT_TYPE)
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_optional_argument(PROXY_KEY, proxy.as_ref())
        .add_attribute(NODE_ID_KEY, mix_id.to_string())
}

pub fn new_gateway_bonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    amount: &Coin,
    identity: IdentityKeyRef<'_>,
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
    identity: IdentityKeyRef<'_>,
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
    identity: IdentityKeyRef<'_>,
    node_id: NodeId,
    assigned_layer: Layer,
) -> Event {
    // coin implements Display trait and we use that implementation here
    Event::new(MixnetEventType::MixnodeBonding)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(OWNER_KEY, owner)
        .add_optional_argument(PROXY_KEY, proxy.as_ref())
        .add_attribute(ASSIGNED_LAYER_KEY, assigned_layer)
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_pending_mixnode_unbonding_event(
    owner: &Addr,
    proxy: &Option<Addr>,
    identity: IdentityKeyRef<'_>,
    node_id: NodeId,
) -> Event {
    // coin implements Display trait and we use that implementation here
    Event::new(MixnetEventType::PendingMixnodeUnbonding)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(OWNER_KEY, owner)
        .add_optional_argument(PROXY_KEY, proxy.as_ref())
}

pub fn new_mixnode_config_update_event(
    node_id: NodeId,
    owner: &Addr,
    proxy: &Option<Addr>,
    update: &MixNodeConfigUpdate,
) -> Event {
    Event::new(MixnetEventType::MixnodeConfigUpdate)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(OWNER_KEY, owner)
        .add_optional_argument(PROXY_KEY, proxy.as_ref())
        .add_attribute(UPDATED_MIXNODE_CONFIG_KEY, update.to_inline_json())
}

pub fn new_mixnode_pending_cost_params_update_event(
    node_id: NodeId,
    owner: &Addr,
    proxy: &Option<Addr>,
    new_costs: &MixNodeCostParams,
) -> Event {
    Event::new(MixnetEventType::PendingMixnodeCostParamsUpdate)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(OWNER_KEY, owner)
        .add_optional_argument(PROXY_KEY, proxy.as_ref())
        .add_attribute(UPDATED_MIXNODE_COST_PARAMS_KEY, new_costs.to_inline_json())
}

pub fn new_rewarding_validator_address_update_event(old: Addr, new: Addr) -> Event {
    Event::new(REWARDING_VALIDATOR_UPDATE_EVENT_TYPE)
        .add_attribute(OLD_REWARDING_VALIDATOR_ADDRESS_KEY, old)
        .add_attribute(NEW_REWARDING_VALIDATOR_ADDRESS_KEY, new)
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
                old_params.minimum_mixnode_pledge.to_string(),
            )
            .add_attribute(
                NEW_MINIMUM_MIXNODE_PLEDGE_KEY,
                new_params.minimum_mixnode_pledge.to_string(),
            )
    }

    if old_params.minimum_gateway_pledge != new_params.minimum_gateway_pledge {
        event = event
            .add_attribute(
                OLD_MINIMUM_GATEWAY_PLEDGE_KEY,
                old_params.minimum_gateway_pledge.to_string(),
            )
            .add_attribute(
                NEW_MINIMUM_GATEWAY_PLEDGE_KEY,
                new_params.minimum_gateway_pledge.to_string(),
            )
    }

    event
}

pub fn new_not_found_mix_operator_rewarding_event(interval: Interval, node_id: NodeId) -> Event {
    Event::new(MixnetEventType::MixnodeRewarding)
        .add_attribute(INTERVAL_KEY, interval.to_string())
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NO_REWARD_REASON_KEY, BOND_NOT_FOUND_VALUE)
}

pub fn new_zero_uptime_mix_operator_rewarding_event(interval: Interval, node_id: NodeId) -> Event {
    Event::new(MixnetEventType::MixnodeRewarding)
        .add_attribute(INTERVAL_KEY, interval.to_string())
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NO_REWARD_REASON_KEY, ZERO_PERFORMANCE_VALUE)
}

pub fn new_mix_rewarding_event(
    interval: Interval,
    node_id: NodeId,
    reward_distribution: RewardDistribution,
) -> Event {
    Event::new(MixnetEventType::MixnodeRewarding)
        .add_attribute(INTERVAL_KEY, interval.to_string())
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(
            OPERATOR_REWARD_KEY,
            reward_distribution.operator.to_string(),
        )
        .add_attribute(
            DELEGATES_REWARD_KEY,
            reward_distribution.delegates.to_string(),
        )
}

pub fn new_mix_delegators_rewarding_event(
    interval_id: u32,
    identity: IdentityKeyRef<'_>,
    delegation_rewards_distributed: Uint128,
    further_delegations: bool,
) -> Event {
    Event::new(MIX_DELEGATORS_REWARDING_EVENT_TYPE)
        .add_attribute(INTERVAL_KEY, interval_id.to_string())
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

// note that when this event is emitted, we'll know the current block height
pub fn new_change_rewarded_set_event(
    active_set_size: u32,
    rewarded_set_size: u32,
    nodes_in_rewarded_set: u32,
) -> Event {
    Event::new(CHANGE_REWARDED_SET_EVENT_TYPE)
        .add_attribute(ACTIVE_SET_SIZE_KEY, active_set_size.to_string())
        .add_attribute(REWARDED_SET_SIZE_KEY, rewarded_set_size.to_string())
        .add_attribute(NODES_IN_REWARDED_SET_KEY, nodes_in_rewarded_set.to_string())
}

pub fn new_advance_epoch_event(interval: Interval) -> Event {
    Event::new(ADVANCE_EPOCH_EVENT_TYPE).add_attribute(NEW_CURRENT_EPOCH_KEY, interval.to_string())
}
