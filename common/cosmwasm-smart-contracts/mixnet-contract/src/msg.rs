// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegation::OwnerProxySubKey;
use crate::error::MixnetContractError;
use crate::helpers::IntoBaseDecimal;
use crate::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use crate::reward_params::{
    IntervalRewardParams, IntervalRewardingParamsUpdate, Performance, RewardingParams,
};
use crate::{delegation, ContractStateParams, MixId, Percent};
use crate::{Gateway, IdentityKey, MixNode};
use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub rewarding_validator_address: String,
    pub vesting_contract_address: String,

    pub rewarding_denom: String,
    pub epochs_in_interval: u32,
    pub epoch_duration: Duration,
    pub initial_rewarding_params: InitialRewardingParams,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitialRewardingParams {
    pub initial_reward_pool: Decimal,
    pub initial_staking_supply: Decimal,

    pub staking_supply_scale_factor: Percent,
    pub sybil_resistance: Percent,
    pub active_set_work_factor: Decimal,
    pub interval_pool_emission: Percent,

    pub rewarded_set_size: u32,
    pub active_set_size: u32,
}

impl InitialRewardingParams {
    pub fn into_rewarding_params(
        self,
        epochs_in_interval: u32,
    ) -> Result<RewardingParams, MixnetContractError> {
        let epoch_reward_budget = self.initial_reward_pool
            / epochs_in_interval.into_base_decimal()?
            * self.interval_pool_emission;
        let stake_saturation_point =
            self.initial_staking_supply / self.rewarded_set_size.into_base_decimal()?;

        Ok(RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: self.initial_reward_pool,
                staking_supply: self.initial_staking_supply,
                staking_supply_scale_factor: self.staking_supply_scale_factor,
                epoch_reward_budget,
                stake_saturation_point,
                sybil_resistance: self.sybil_resistance,
                active_set_work_factor: self.active_set_work_factor,
                interval_pool_emission: self.interval_pool_emission,
            },
            rewarded_set_size: self.rewarded_set_size,
            active_set_size: self.active_set_size,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // state/sys-params-related
    UpdateRewardingValidatorAddress {
        address: String,
    },
    UpdateContractStateParams {
        updated_parameters: ContractStateParams,
    },
    UpdateActiveSetSize {
        active_set_size: u32,
        force_immediately: bool,
    },
    UpdateRewardingParams {
        updated_params: IntervalRewardingParamsUpdate,
        force_immediately: bool,
    },
    UpdateIntervalConfig {
        epochs_in_interval: u32,
        epoch_duration_secs: u64,
        force_immediately: bool,
    },
    AdvanceCurrentEpoch {
        new_rewarded_set: Vec<MixId>,
        expected_active_set_size: u32,
    },
    ReconcileEpochEvents {
        limit: Option<u32>,
    },

    // mixnode-related:
    BondMixnode {
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: String,
    },
    BondMixnodeOnBehalf {
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: String,
        owner: String,
    },
    UnbondMixnode {},
    UnbondMixnodeOnBehalf {
        owner: String,
    },
    UpdateMixnodeCostParams {
        new_costs: MixNodeCostParams,
    },
    UpdateMixnodeCostParamsOnBehalf {
        new_costs: MixNodeCostParams,
        owner: String,
    },
    UpdateMixnodeConfig {
        new_config: MixNodeConfigUpdate,
    },
    UpdateMixnodeConfigOnBehalf {
        new_config: MixNodeConfigUpdate,
        owner: String,
    },

    // gateway-related:
    BondGateway {
        gateway: Gateway,
        owner_signature: String,
    },
    BondGatewayOnBehalf {
        gateway: Gateway,
        owner: String,
        owner_signature: String,
    },
    UnbondGateway {},
    UnbondGatewayOnBehalf {
        owner: String,
    },

    // delegation-related:
    DelegateToMixnode {
        mix_id: MixId,
    },
    DelegateToMixnodeOnBehalf {
        mix_id: MixId,
        delegate: String,
    },
    UndelegateFromMixnode {
        mix_id: MixId,
    },
    UndelegateFromMixnodeOnBehalf {
        mix_id: MixId,
        delegate: String,
    },

    // reward-related
    RewardMixnode {
        mix_id: MixId,
        performance: Performance,
    },
    WithdrawOperatorReward {},
    WithdrawOperatorRewardOnBehalf {
        owner: String,
    },
    WithdrawDelegatorReward {
        mix_id: MixId,
    },
    WithdrawDelegatorRewardOnBehalf {
        mix_id: MixId,
        owner: String,
    },

    // testing-only
    #[cfg(feature = "contract-testing")]
    TestingResolveAllPendingEvents {
        limit: Option<u32>,
    },
}

impl ExecuteMsg {
    pub fn default_memo(&self) -> String {
        match self {
            ExecuteMsg::UpdateRewardingValidatorAddress { address } => {
                format!("updating rewarding validator to {}", address)
            }
            ExecuteMsg::UpdateContractStateParams { .. } => {
                "updating mixnet state parameters".into()
            }
            ExecuteMsg::UpdateActiveSetSize {
                active_set_size,
                force_immediately,
            } => format!(
                "updating active set size to {}. forced: {}",
                active_set_size, force_immediately
            ),
            ExecuteMsg::UpdateRewardingParams {
                force_immediately, ..
            } => format!(
                "updating mixnet rewarding parameters. forced: {}",
                force_immediately
            ),
            ExecuteMsg::UpdateIntervalConfig {
                force_immediately, ..
            } => format!(
                "updating mixnet interval configuration. forced: {}",
                force_immediately
            ),
            ExecuteMsg::AdvanceCurrentEpoch { .. } => "advancing current epoch".into(),
            ExecuteMsg::ReconcileEpochEvents { .. } => "reconciling epoch events".into(),
            ExecuteMsg::BondMixnode { mix_node, .. } => {
                format!("bonding mixnode {}", mix_node.identity_key)
            }
            ExecuteMsg::BondMixnodeOnBehalf { mix_node, .. } => {
                format!("bonding mixnode {} on behalf", mix_node.identity_key)
            }
            ExecuteMsg::UnbondMixnode { .. } => "unbonding mixnode".into(),
            ExecuteMsg::UnbondMixnodeOnBehalf { .. } => "unbonding mixnode on behalf".into(),
            ExecuteMsg::UpdateMixnodeCostParams { .. } => "updating mixnode cost parameters".into(),
            ExecuteMsg::UpdateMixnodeCostParamsOnBehalf { .. } => {
                "updating mixnode cost parameters on behalf".into()
            }
            ExecuteMsg::UpdateMixnodeConfig { .. } => "updating mixnode configuration".into(),
            ExecuteMsg::UpdateMixnodeConfigOnBehalf { .. } => {
                "updating mixnode configuration on behalf".into()
            }
            ExecuteMsg::BondGateway { gateway, .. } => {
                format!("bonding gateway {}", gateway.identity_key)
            }
            ExecuteMsg::BondGatewayOnBehalf { gateway, .. } => {
                format!("bonding gateway {} on behalf", gateway.identity_key)
            }
            ExecuteMsg::UnbondGateway { .. } => "unbonding gateway".into(),
            ExecuteMsg::UnbondGatewayOnBehalf { .. } => "unbonding gateway on behalf".into(),
            ExecuteMsg::DelegateToMixnode { mix_id } => format!("delegating to mixnode {}", mix_id),
            ExecuteMsg::DelegateToMixnodeOnBehalf { mix_id, .. } => {
                format!("delegating to mixnode {} on behalf", mix_id)
            }
            ExecuteMsg::UndelegateFromMixnode { mix_id } => {
                format!("removing delegation from mixnode {}", mix_id)
            }
            ExecuteMsg::UndelegateFromMixnodeOnBehalf { mix_id, .. } => {
                format!("removing delegation from mixnode {} on behalf", mix_id)
            }
            ExecuteMsg::RewardMixnode {
                mix_id,
                performance,
            } => format!(
                "rewarding mixnode {} for performance {}",
                mix_id, performance
            ),
            ExecuteMsg::WithdrawOperatorReward { .. } => "withdrawing operator reward".into(),
            ExecuteMsg::WithdrawOperatorRewardOnBehalf { .. } => {
                "withdrawing operator reward on behalf".into()
            }
            ExecuteMsg::WithdrawDelegatorReward { mix_id } => {
                format!("withdrawing delegator reward from mixnode {}", mix_id)
            }
            ExecuteMsg::WithdrawDelegatorRewardOnBehalf { mix_id, .. } => format!(
                "withdrawing delegator reward from mixnode {} on behalf",
                mix_id
            ),
            #[cfg(feature = "contract-testing")]
            ExecuteMsg::TestingResolveAllPendingEvents { .. } => {
                "resolving all pending events".into()
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // state/sys-params-related
    GetContractVersion {},
    GetRewardingValidatorAddress {},
    GetStateParams {},
    GetState {},
    GetRewardingParams {},
    GetCurrentIntervalDetails {},
    GetRewardedSet {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },

    // mixnode-related:
    GetMixNodeBonds {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },
    GetMixNodesDetailed {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },
    GetUnbondedMixNodes {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },
    GetUnbondedMixNodesByOwner {
        owner: String,
        limit: Option<u32>,
        start_after: Option<MixId>,
    },
    GetUnbondedMixNodesByIdentityKey {
        identity_key: String,
        limit: Option<u32>,
        start_after: Option<MixId>,
    },
    GetOwnedMixnode {
        address: String,
    },
    GetMixnodeDetails {
        mix_id: MixId,
    },
    GetMixnodeRewardingDetails {
        mix_id: MixId,
    },
    GetStakeSaturation {
        mix_id: MixId,
    },
    GetUnbondedMixNodeInformation {
        mix_id: MixId,
    },
    GetBondedMixnodeDetailsByIdentity {
        mix_identity: IdentityKey,
    },
    GetLayerDistribution {},

    // gateway-related:
    GetGateways {
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },
    GetGatewayBond {
        identity: IdentityKey,
    },
    GetOwnedGateway {
        address: String,
    },

    // delegation-related:
    // gets all [paged] delegations associated with particular mixnode
    GetMixnodeDelegations {
        mix_id: MixId,
        // since `start_after` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // gets all [paged] delegations associated with particular delegator
    GetDelegatorDelegations {
        // since `delegator` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        delegator: String,
        start_after: Option<(MixId, OwnerProxySubKey)>,
        limit: Option<u32>,
    },
    // gets delegation associated with particular mixnode, delegator pair
    GetDelegationDetails {
        mix_id: MixId,
        delegator: String,
        proxy: Option<String>,
    },
    // gets all delegations in the system
    GetAllDelegations {
        start_after: Option<delegation::StorageKey>,
        limit: Option<u32>,
    },

    // rewards related
    GetPendingOperatorReward {
        address: String,
    },
    GetPendingMixNodeOperatorReward {
        mix_id: MixId,
    },
    GetPendingDelegatorReward {
        address: String,
        mix_id: MixId,
        proxy: Option<String>,
    },
    // given the provided performance, estimate the reward at the end of the current epoch
    GetEstimatedCurrentEpochOperatorReward {
        mix_id: MixId,
        estimated_performance: Performance,
    },
    GetEstimatedCurrentEpochDelegatorReward {
        address: String,
        mix_id: MixId,
        proxy: Option<String>,
        estimated_performance: Performance,
    },

    // interval-related
    GetPendingEpochEvents {
        limit: Option<u32>,
        start_after: Option<u32>,
    },
    GetPendingIntervalEvents {
        limit: Option<u32>,
        start_after: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub vesting_contract_address: String,
}
