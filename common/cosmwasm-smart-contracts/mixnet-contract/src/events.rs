// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::gateway::GatewayConfigUpdate;
use crate::mixnode::{MixNodeConfigUpdate, NodeCostParams};
use crate::nym_node::Role;
use crate::reward_params::{ActiveSetUpdate, IntervalRewardParams, IntervalRewardingParamsUpdate};
use crate::rewarding::RewardDistribution;
use crate::{BlockHeight, ContractStateParams, EpochId, IdentityKeyRef, Interval, NodeId};
pub use contracts_common::events::*;
use cosmwasm_std::{Addr, Coin, Decimal, Event};
use std::fmt::Display;

pub const EVENT_VERSION_PREFIX: &str = "v2_";

pub enum MixnetEventType {
    MixnodeBonding,
    NymNodeBonding,
    NymNodeUnbonding,
    PendingNymNodeUnbonding,
    GatewayMigration,
    MixnodeMigration,
    PendingPledgeIncrease,
    PledgeIncrease,
    PendingPledgeDecrease,
    PledgeDecrease,
    GatewayBonding,
    GatewayUnbonding,
    PendingMixnodeUnbonding,
    MixnodeUnbonding,
    MixnodeConfigUpdate,
    PendingCostParamsUpdate,
    CostParamsUpdate,
    NodeRewarding,
    WithdrawDelegatorReward,
    WithdrawOperatorReward,
    PendingActiveSetUpdate,
    ActiveSetUpdate,
    PendingIntervalRewardingParamsUpdate,
    IntervalRewardingParamsUpdate,
    PendingDelegation,
    PendingUndelegation,
    Delegation,
    DelegationOnUnbonding,
    Undelegation,
    ContractSettingsUpdate,
    RewardingValidatorUpdate,
    BeginEpochTransition,
    AdvanceEpoch,
    RoleAssignment,
    ExecutePendingEpochEvents,
    ExecutePendingIntervalEvents,
    ReconcilePendingEvents,
    PendingIntervalConfigUpdate,
    IntervalConfigUpdate,
    GatewayConfigUpdate,
}

impl From<MixnetEventType> for String {
    fn from(typ: MixnetEventType) -> Self {
        typ.to_string()
    }
}

impl Display for MixnetEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let event_name = match self {
            MixnetEventType::MixnodeBonding => "mixnode_bonding",
            MixnetEventType::NymNodeBonding => "nymnode_bonding",
            MixnetEventType::NymNodeUnbonding => "nymnode_unbonding",
            MixnetEventType::PendingNymNodeUnbonding => "pending_nymnode_unbonding",
            MixnetEventType::GatewayMigration => "gateway_migration",
            MixnetEventType::MixnodeMigration => "mixnode_migration",
            MixnetEventType::PendingPledgeIncrease => "pending_pledge_increase",
            MixnetEventType::PledgeIncrease => "pledge_increase",
            MixnetEventType::PendingPledgeDecrease => "pending_pledge_decrease",
            MixnetEventType::PledgeDecrease => "pledge_decrease",
            MixnetEventType::GatewayBonding => "gateway_bonding",
            MixnetEventType::GatewayUnbonding => "gateway_unbonding",
            MixnetEventType::PendingMixnodeUnbonding => "pending_mixnode_unbonding",
            MixnetEventType::MixnodeConfigUpdate => "mixnode_config_update",
            MixnetEventType::MixnodeUnbonding => "mixnode_unbonding",
            MixnetEventType::PendingCostParamsUpdate => "pending_cost_params_update",
            MixnetEventType::CostParamsUpdate => "cost_params_update",
            MixnetEventType::NodeRewarding => "node_rewarding",
            MixnetEventType::WithdrawDelegatorReward => "withdraw_delegator_reward",
            MixnetEventType::WithdrawOperatorReward => "withdraw_operator_reward",
            MixnetEventType::PendingActiveSetUpdate => "pending_active_set_update",
            MixnetEventType::ActiveSetUpdate => "active_set_update",
            MixnetEventType::PendingIntervalRewardingParamsUpdate => {
                "pending_interval_rewarding_params_update"
            }
            MixnetEventType::IntervalRewardingParamsUpdate => "interval_rewarding_params_update",
            MixnetEventType::PendingDelegation => "pending_delegation",
            MixnetEventType::PendingUndelegation => "pending_undelegation",
            MixnetEventType::Delegation => "delegation",
            MixnetEventType::Undelegation => "undelegation",
            MixnetEventType::ContractSettingsUpdate => "settings_update",
            MixnetEventType::RewardingValidatorUpdate => "rewarding_validator_address_update",
            MixnetEventType::BeginEpochTransition => "beginning_epoch_transition",
            MixnetEventType::AdvanceEpoch => "advance_epoch",
            MixnetEventType::RoleAssignment => "role_assignment",
            MixnetEventType::ExecutePendingEpochEvents => "execute_pending_epoch_events",
            MixnetEventType::ExecutePendingIntervalEvents => "execute_pending_interval_events",
            MixnetEventType::ReconcilePendingEvents => "reconcile_pending_events",
            MixnetEventType::PendingIntervalConfigUpdate => "pending_interval_config_update",
            MixnetEventType::IntervalConfigUpdate => "interval_config_update",
            MixnetEventType::DelegationOnUnbonding => "delegation_on_unbonding_node",
            MixnetEventType::GatewayConfigUpdate => "gateway_config_update",
        };

        write!(f, "{EVENT_VERSION_PREFIX}{event_name}")
    }
}

// attributes that are used in multiple places
pub const OWNER_KEY: &str = "owner";
pub const AMOUNT_KEY: &str = "amount";
pub const ERROR_MESSAGE_KEY: &str = "error_message";

// event-specific attributes

// delegation/undelegation
pub const DELEGATOR_KEY: &str = "delegator";
pub const DELEGATION_TARGET_KEY: &str = "delegation_target";
pub const UNIT_REWARD_KEY: &str = "unit_reward";

// bonding/unbonding
pub const MIX_ID_KEY: &str = "mix_id";
pub const NODE_ID_KEY: &str = "node_id";
pub const NODE_IDENTITY_KEY: &str = "identity";

// settings change
pub const OLD_MINIMUM_PLEDGE_KEY: &str = "old_minimum_pledge";
pub const OLD_MINIMUM_DELEGATION_KEY: &str = "old_minimum_delegation";

pub const NEW_MINIMUM_PLEDGE_KEY: &str = "new_minimum_pledge";
pub const NEW_MINIMUM_DELEGATION_KEY: &str = "new_minimum_delegation";

pub const OLD_REWARDING_VALIDATOR_ADDRESS_KEY: &str = "old_rewarding_validator_address";
pub const NEW_REWARDING_VALIDATOR_ADDRESS_KEY: &str = "new_rewarding_validator_address";

pub const UPDATED_MIXNODE_CONFIG_KEY: &str = "updated_mixnode_config";
pub const UPDATED_GATEWAY_CONFIG_KEY: &str = "updated_gateway_config";
pub const UPDATED_MIXNODE_COST_PARAMS_KEY: &str = "updated_mixnode_cost_params";

// rewarding
pub const INTERVAL_KEY: &str = "interval_details";
pub const OPERATOR_REWARD_KEY: &str = "operator_reward";
pub const DELEGATES_REWARD_KEY: &str = "delegates_reward";
pub const APPROXIMATE_TIME_LEFT_SECS_KEY: &str = "approximate_time_left_secs";
pub const INTERVAL_REWARDING_PARAMS_UPDATE_KEY: &str = "interval_rewarding_params_update";
pub const UPDATED_INTERVAL_REWARDING_PARAMS_KEY: &str = "updated_interval_rewarding_params";
pub const PRIOR_DELEGATES_KEY: &str = "prior_delegates";
pub const PRIOR_UNIT_REWARD_KEY: &str = "prior_unit_reward";

pub const NO_REWARD_REASON_KEY: &str = "no_reward_reason";
pub const BOND_NOT_FOUND_VALUE: &str = "bond_not_found";
pub const ZERO_PERFORMANCE_OR_WORK_VALUE: &str = "zero_performance_or_work";

// rewarded set update
pub const NUM_MIXNODES_KEY: &str = "num_mixnodes";
pub const NUM_ENTRIES_KEY: &str = "num_entry_gateways";
pub const NUM_EXITS_KEY: &str = "num_exit_gateways";

pub const CURRENT_EPOCH_KEY: &str = "current_epoch";
pub const NEW_CURRENT_EPOCH_KEY: &str = "new_current_epoch";

pub const ROLE_KEY: &str = "role";
pub const NODE_COUNT_KEY: &str = "node_count";

// interval
pub const EVENTS_EXECUTED_KEY: &str = "number_of_events_executed";
pub const EVENT_CREATION_HEIGHT_KEY: &str = "created_at";
pub const REWARDED_SET_NODES_KEY: &str = "rewarded_set_nodes";
pub const NEW_EPOCHS_DURATION_SECS_KEY: &str = "new_epoch_durations_secs";
pub const NEW_EPOCHS_IN_INTERVAL: &str = "new_epochs_in_interval";

pub fn new_delegation_event(
    created_at: BlockHeight,
    delegator: &Addr,
    amount: &Coin,
    mix_id: NodeId,
    unit_reward: Decimal,
) -> Event {
    Event::new(MixnetEventType::Delegation)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_id.to_string())
        .add_attribute(UNIT_REWARD_KEY, unit_reward.to_string())
}

pub fn new_delegation_on_unbonded_node_event(delegator: &Addr, mix_id: NodeId) -> Event {
    Event::new(MixnetEventType::Delegation)
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_attribute(DELEGATION_TARGET_KEY, mix_id.to_string())
}

pub fn new_pending_delegation_event(delegator: &Addr, amount: &Coin, mix_id: NodeId) -> Event {
    Event::new(MixnetEventType::PendingDelegation)
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_id.to_string())
}

pub fn new_withdraw_operator_reward_event(owner: &Addr, amount: Coin, mix_id: NodeId) -> Event {
    Event::new(MixnetEventType::WithdrawOperatorReward)
        .add_attribute(OWNER_KEY, owner.as_str())
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(MIX_ID_KEY, mix_id.to_string())
}

pub fn new_withdraw_delegator_reward_event(
    delegator: &Addr,
    amount: Coin,
    mix_id: NodeId,
) -> Event {
    Event::new(MixnetEventType::WithdrawDelegatorReward)
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_attribute(AMOUNT_KEY, amount.to_string())
        .add_attribute(DELEGATION_TARGET_KEY, mix_id.to_string())
}

pub fn new_active_set_update_failure(err: MixnetContractError) -> Event {
    Event::new(MixnetEventType::ActiveSetUpdate).add_attribute(ERROR_MESSAGE_KEY, err.to_string())
}

pub fn new_active_set_update_event(created_at: BlockHeight, update: ActiveSetUpdate) -> Event {
    Event::new(MixnetEventType::ActiveSetUpdate)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(NUM_MIXNODES_KEY, update.mixnodes.to_string())
        .add_attribute(NUM_ENTRIES_KEY, update.entry_gateways.to_string())
        .add_attribute(NUM_EXITS_KEY, update.exit_gateways.to_string())
}

pub fn new_pending_active_set_update_event(
    update: ActiveSetUpdate,
    approximate_time_remaining_secs: i64,
) -> Event {
    Event::new(MixnetEventType::PendingActiveSetUpdate)
        .add_attribute(NUM_MIXNODES_KEY, update.mixnodes.to_string())
        .add_attribute(NUM_ENTRIES_KEY, update.entry_gateways.to_string())
        .add_attribute(NUM_EXITS_KEY, update.exit_gateways.to_string())
        .add_attribute(
            APPROXIMATE_TIME_LEFT_SECS_KEY,
            approximate_time_remaining_secs.to_string(),
        )
}

pub fn new_rewarding_params_update_event(
    created_at: BlockHeight,
    update: IntervalRewardingParamsUpdate,
    updated: IntervalRewardParams,
) -> Event {
    Event::new(MixnetEventType::IntervalRewardingParamsUpdate)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(
            INTERVAL_REWARDING_PARAMS_UPDATE_KEY,
            update.to_inline_json(),
        )
        .add_attribute(
            UPDATED_INTERVAL_REWARDING_PARAMS_KEY,
            updated.to_inline_json(),
        )
}

pub fn new_pending_rewarding_params_update_event(
    update: IntervalRewardingParamsUpdate,
    approximate_time_remaining_secs: i64,
) -> Event {
    Event::new(MixnetEventType::PendingIntervalRewardingParamsUpdate)
        .add_attribute(
            INTERVAL_REWARDING_PARAMS_UPDATE_KEY,
            update.to_inline_json(),
        )
        .add_attribute(
            APPROXIMATE_TIME_LEFT_SECS_KEY,
            approximate_time_remaining_secs.to_string(),
        )
}

pub fn new_undelegation_event(created_at: BlockHeight, delegator: &Addr, mix_id: NodeId) -> Event {
    Event::new(MixnetEventType::Undelegation)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_attribute(MIX_ID_KEY, mix_id.to_string())
}

pub fn new_pending_undelegation_event(delegator: &Addr, mix_id: NodeId) -> Event {
    Event::new(MixnetEventType::PendingUndelegation)
        .add_attribute(DELEGATOR_KEY, delegator)
        .add_attribute(MIX_ID_KEY, mix_id.to_string())
}

pub fn new_gateway_unbonding_event(
    owner: &Addr,
    amount: &Coin,
    identity: IdentityKeyRef<'_>,
) -> Event {
    Event::new(MixnetEventType::GatewayUnbonding)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_nym_node_bonding_event(
    owner: &Addr,
    amount: &Coin,
    identity: IdentityKeyRef<'_>,
    node_id: NodeId,
) -> Event {
    Event::new(MixnetEventType::NymNodeBonding)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_nym_node_unbonding_event(created_at: BlockHeight, node_id: NodeId) -> Event {
    Event::new(MixnetEventType::NymNodeUnbonding)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(NODE_ID_KEY, node_id.to_string())
}

pub fn new_pending_nym_node_unbonding_event(
    owner: &Addr,
    identity: IdentityKeyRef<'_>,
    node_id: NodeId,
) -> Event {
    Event::new(MixnetEventType::PendingNymNodeUnbonding)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(OWNER_KEY, owner)
}

pub fn new_migrated_gateway_event(
    owner: &Addr,
    identity: IdentityKeyRef<'_>,
    node_id: NodeId,
) -> Event {
    Event::new(MixnetEventType::GatewayMigration)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(OWNER_KEY, owner)
}

pub fn new_migrated_mixnode_event(
    owner: &Addr,
    identity: IdentityKeyRef<'_>,
    node_id: NodeId,
) -> Event {
    Event::new(MixnetEventType::MixnodeMigration)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(OWNER_KEY, owner)
}

pub fn new_pending_pledge_increase_event(node_id: NodeId, amount: &Coin) -> Event {
    Event::new(MixnetEventType::PendingPledgeIncrease)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_pledge_increase_event(created_at: BlockHeight, node_id: NodeId, amount: &Coin) -> Event {
    Event::new(MixnetEventType::PledgeIncrease)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_pending_pledge_decrease_event(node_id: NodeId, amount: &Coin) -> Event {
    Event::new(MixnetEventType::PendingPledgeDecrease)
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_pledge_decrease_event(created_at: BlockHeight, node_id: NodeId, amount: &Coin) -> Event {
    Event::new(MixnetEventType::PledgeDecrease)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(AMOUNT_KEY, amount.to_string())
}

pub fn new_mixnode_unbonding_event(created_at: BlockHeight, mix_id: NodeId) -> Event {
    Event::new(MixnetEventType::MixnodeUnbonding)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(MIX_ID_KEY, mix_id.to_string())
}

pub fn new_pending_mixnode_unbonding_event(
    owner: &Addr,
    identity: IdentityKeyRef<'_>,
    mix_id: NodeId,
) -> Event {
    Event::new(MixnetEventType::PendingMixnodeUnbonding)
        .add_attribute(MIX_ID_KEY, mix_id.to_string())
        .add_attribute(NODE_IDENTITY_KEY, identity)
        .add_attribute(OWNER_KEY, owner)
}

pub fn new_mixnode_config_update_event(
    mix_id: NodeId,
    owner: &Addr,
    update: &MixNodeConfigUpdate,
) -> Event {
    Event::new(MixnetEventType::MixnodeConfigUpdate)
        .add_attribute(MIX_ID_KEY, mix_id.to_string())
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(UPDATED_MIXNODE_CONFIG_KEY, update.to_inline_json())
}

pub fn new_gateway_config_update_event(owner: &Addr, update: &GatewayConfigUpdate) -> Event {
    Event::new(MixnetEventType::GatewayConfigUpdate)
        .add_attribute(OWNER_KEY, owner)
        .add_attribute(UPDATED_GATEWAY_CONFIG_KEY, update.to_inline_json())
}

pub fn new_pending_cost_params_update_event(mix_id: NodeId, new_costs: &NodeCostParams) -> Event {
    Event::new(MixnetEventType::PendingCostParamsUpdate)
        .add_attribute(NODE_ID_KEY, mix_id.to_string())
        .add_attribute(UPDATED_MIXNODE_COST_PARAMS_KEY, new_costs.to_inline_json())
}

pub fn new_cost_params_update_event(
    created_at: BlockHeight,
    mix_id: NodeId,
    new_costs: &NodeCostParams,
) -> Event {
    Event::new(MixnetEventType::CostParamsUpdate)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(MIX_ID_KEY, mix_id.to_string())
        .add_attribute(UPDATED_MIXNODE_COST_PARAMS_KEY, new_costs.to_inline_json())
}

pub fn new_rewarding_validator_address_update_event(old: Addr, new: Addr) -> Event {
    Event::new(MixnetEventType::RewardingValidatorUpdate)
        .add_attribute(OLD_REWARDING_VALIDATOR_ADDRESS_KEY, old)
        .add_attribute(NEW_REWARDING_VALIDATOR_ADDRESS_KEY, new)
}

pub fn new_settings_update_event(
    old_params: &ContractStateParams,
    new_params: &ContractStateParams,
) -> Event {
    let mut event = Event::new(MixnetEventType::ContractSettingsUpdate);

    if old_params.minimum_pledge != new_params.minimum_pledge {
        event = event
            .add_attribute(
                OLD_MINIMUM_PLEDGE_KEY,
                old_params.minimum_pledge.to_string(),
            )
            .add_attribute(
                NEW_MINIMUM_PLEDGE_KEY,
                new_params.minimum_pledge.to_string(),
            )
    }

    if old_params.minimum_delegation != new_params.minimum_delegation {
        if let Some(ref old) = old_params.minimum_delegation {
            event = event.add_attribute(OLD_MINIMUM_DELEGATION_KEY, old.to_string())
        } else {
            event = event.add_attribute(OLD_MINIMUM_DELEGATION_KEY, "None")
        }
        if let Some(ref new) = new_params.minimum_delegation {
            event = event.add_attribute(NEW_MINIMUM_DELEGATION_KEY, new.to_string())
        } else {
            event = event.add_attribute(NEW_MINIMUM_DELEGATION_KEY, "None")
        }
    }

    event
}

pub fn new_not_found_node_operator_rewarding_event(interval: Interval, node_id: NodeId) -> Event {
    Event::new(MixnetEventType::NodeRewarding)
        .add_attribute(
            INTERVAL_KEY,
            interval.current_epoch_absolute_id().to_string(),
        )
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NO_REWARD_REASON_KEY, BOND_NOT_FOUND_VALUE)
}

pub fn new_zero_uptime_mix_operator_rewarding_event(interval: Interval, node_id: NodeId) -> Event {
    Event::new(MixnetEventType::NodeRewarding)
        .add_attribute(
            INTERVAL_KEY,
            interval.current_epoch_absolute_id().to_string(),
        )
        .add_attribute(NODE_ID_KEY, node_id.to_string())
        .add_attribute(NO_REWARD_REASON_KEY, ZERO_PERFORMANCE_OR_WORK_VALUE)
}

pub fn new_mix_rewarding_event(
    interval: Interval,
    node_id: NodeId,
    reward_distribution: RewardDistribution,
    prior_delegates: Decimal,
    prior_unit_reward: Decimal,
) -> Event {
    Event::new(MixnetEventType::NodeRewarding)
        .add_attribute(
            INTERVAL_KEY,
            interval.current_epoch_absolute_id().to_string(),
        )
        .add_attribute(PRIOR_DELEGATES_KEY, prior_delegates.to_string())
        .add_attribute(PRIOR_UNIT_REWARD_KEY, prior_unit_reward.to_string())
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

pub fn new_epoch_transition_start_event(current_interval: Interval) -> Event {
    Event::new(MixnetEventType::BeginEpochTransition).add_attribute(
        CURRENT_EPOCH_KEY,
        current_interval.current_epoch_absolute_id().to_string(),
    )
}

pub fn new_assigned_role_event(role: Role, nodes: u32) -> Event {
    Event::new(MixnetEventType::RoleAssignment)
        .add_attribute(ROLE_KEY, role.to_string())
        .add_attribute(NODE_COUNT_KEY, nodes.to_string())
}

pub fn new_advance_epoch_event(epoch_id: EpochId) -> Event {
    Event::new(MixnetEventType::AdvanceEpoch)
        .add_attribute(NEW_CURRENT_EPOCH_KEY, epoch_id.to_string())
}

pub fn new_pending_epoch_events_execution_event(executed: u32) -> Event {
    Event::new(MixnetEventType::ExecutePendingEpochEvents)
        .add_attribute(EVENTS_EXECUTED_KEY, executed.to_string())
}

pub fn new_pending_interval_events_execution_event(executed: u32) -> Event {
    Event::new(MixnetEventType::ExecutePendingIntervalEvents)
        .add_attribute(EVENTS_EXECUTED_KEY, executed.to_string())
}

pub fn new_reconcile_pending_events() -> Event {
    Event::new(MixnetEventType::ReconcilePendingEvents)
}

pub fn new_interval_config_update_event(
    created_at: BlockHeight,
    epochs_in_interval: u32,
    epoch_duration_secs: u64,
    updated_rewarding_params: IntervalRewardParams,
) -> Event {
    Event::new(MixnetEventType::IntervalConfigUpdate)
        .add_attribute(EVENT_CREATION_HEIGHT_KEY, created_at.to_string())
        .add_attribute(
            NEW_EPOCHS_DURATION_SECS_KEY,
            epoch_duration_secs.to_string(),
        )
        .add_attribute(NEW_EPOCHS_IN_INTERVAL, epochs_in_interval.to_string())
        .add_attribute(
            UPDATED_INTERVAL_REWARDING_PARAMS_KEY,
            updated_rewarding_params.to_inline_json(),
        )
}

pub fn new_pending_interval_config_update_event(
    epochs_in_interval: u32,
    epoch_duration_secs: u64,
    approximate_time_remaining_secs: i64,
) -> Event {
    Event::new(MixnetEventType::PendingIntervalConfigUpdate)
        .add_attribute(
            NEW_EPOCHS_DURATION_SECS_KEY,
            epoch_duration_secs.to_string(),
        )
        .add_attribute(NEW_EPOCHS_IN_INTERVAL, epochs_in_interval.to_string())
        .add_attribute(
            APPROXIMATE_TIME_LEFT_SECS_KEY,
            approximate_time_remaining_secs.to_string(),
        )
}
