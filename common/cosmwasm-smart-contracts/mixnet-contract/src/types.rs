// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config_score::{ConfigScoreParams, OutdatedVersionWeights, VersionScoreFormulaParams};
use crate::nym_node::Role;
use crate::reward_params::RewardedSetParams;
use crate::EpochId;
use contracts_common::Percent;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use cosmwasm_std::{Addr, Uint128};
use std::fmt::{Display, Formatter};

// type aliases for better reasoning about available data
pub type SphinxKey = String;
pub type SphinxKeyRef<'a> = &'a str;

pub type NodeId = u32;
pub type BlockHeight = u64;

#[cw_serde]
pub struct RoleAssignment {
    pub role: Role,
    pub nodes: Vec<NodeId>,
}

impl RoleAssignment {
    pub fn new(role: Role, nodes: Vec<NodeId>) -> RoleAssignment {
        RoleAssignment { role, nodes }
    }

    pub fn is_final_assignment(&self) -> bool {
        self.role.is_standby()
    }
}

#[cw_serde]
#[derive(Default)]
pub struct EpochRewardedSet {
    pub epoch_id: EpochId,

    pub assignment: RewardedSet,
}

impl From<(EpochId, RewardedSet)> for EpochRewardedSet {
    fn from((epoch_id, assignment): (EpochId, RewardedSet)) -> Self {
        EpochRewardedSet {
            epoch_id,
            assignment,
        }
    }
}

#[cw_serde]
#[derive(Default)]
pub struct RewardedSet {
    pub entry_gateways: Vec<NodeId>,

    pub exit_gateways: Vec<NodeId>,

    pub layer1: Vec<NodeId>,

    pub layer2: Vec<NodeId>,

    pub layer3: Vec<NodeId>,

    pub standby: Vec<NodeId>,
}

impl RewardedSet {
    pub fn is_empty(&self) -> bool {
        self.entry_gateways.is_empty()
            && self.exit_gateways.is_empty()
            && self.layer1.is_empty()
            && self.layer2.is_empty()
            && self.layer3.is_empty()
            && self.standby.is_empty()
    }

    pub fn active_set_size(&self) -> usize {
        self.entry_gateways.len()
            + self.exit_gateways.len()
            + self.layer1.len()
            + self.layer2.len()
            + self.layer3.len()
    }

    pub fn rewarded_set_size(&self) -> usize {
        self.active_set_size() + self.standby_set_size()
    }

    pub fn standby_set_size(&self) -> usize {
        self.standby.len()
    }

    pub fn get_role(&self, node_id: NodeId) -> Option<Role> {
        // given each role has ~100 entries in them, doing linear lookup with vec should be fine
        if self.entry_gateways.contains(&node_id) {
            return Some(Role::EntryGateway);
        }
        if self.exit_gateways.contains(&node_id) {
            return Some(Role::ExitGateway);
        }
        if self.layer1.contains(&node_id) {
            return Some(Role::Layer1);
        }
        if self.layer2.contains(&node_id) {
            return Some(Role::Layer2);
        }
        if self.layer3.contains(&node_id) {
            return Some(Role::Layer3);
        }
        if self.standby.contains(&node_id) {
            return Some(Role::Standby);
        }
        None
    }

    pub fn matches_parameters(&self, params: RewardedSetParams) -> bool {
        self.entry_gateways.len() <= params.entry_gateways as usize
            && self.exit_gateways.len() <= params.exit_gateways as usize
            && self.layer1.len() + self.layer2.len() + self.layer3.len() <= params.mixnodes as usize
            && self.standby.len() <= params.standby as usize
    }
}

#[cw_serde]
pub struct RangedValue<T> {
    pub minimum: T,
    pub maximum: T,
}

impl<T> Copy for RangedValue<T> where T: Copy {}

pub type ProfitMarginRange = RangedValue<Percent>;
pub type OperatingCostRange = RangedValue<Uint128>;

impl<T> Display for RangedValue<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.minimum, self.maximum)
    }
}

impl Default for ProfitMarginRange {
    fn default() -> Self {
        ProfitMarginRange {
            minimum: Percent::zero(),
            maximum: Percent::hundred(),
        }
    }
}

impl Default for OperatingCostRange {
    fn default() -> Self {
        OperatingCostRange {
            minimum: Uint128::zero(),

            // 1 billion (native tokens, i.e. 1 billion * 1'000'000 base tokens) - the total supply
            maximum: Uint128::new(1_000_000_000_000_000),
        }
    }
}

impl<T> RangedValue<T>
where
    T: Copy + PartialOrd + PartialEq,
{
    pub fn new(minimum: T, maximum: T) -> Self {
        RangedValue { minimum, maximum }
    }

    pub fn normalise(&self, value: T) -> T {
        if value < self.minimum {
            self.minimum
        } else if value > self.maximum {
            self.maximum
        } else {
            value
        }
    }

    pub fn within_range(&self, value: T) -> bool {
        value >= self.minimum && value <= self.maximum
    }
}

#[cfg(feature = "utoipa")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(title = "Coin"))]
pub struct CoinSchema {
    pub denom: String,
    pub amount: String,
}

/// The current state of the mixnet contract.
#[cw_serde]
pub struct ContractState {
    /// Address of the contract owner.
    #[deprecated(
        note = "use explicit ADMIN instead. this field will be removed in future release"
    )]
    #[serde(default)]
    pub owner: Option<Addr>,

    /// Address of "rewarding validator" (nym-api) that's allowed to send any rewarding-related transactions.
    pub rewarding_validator_address: Addr,

    /// Address of the vesting contract to which the mixnet contract would be sending all
    /// track-related messages.
    pub vesting_contract_address: Addr,

    /// The expected denom used for rewarding (and realistically any other operation).
    /// Default: `unym`
    pub rewarding_denom: String,

    /// Contract parameters that could be adjusted in a transaction the contract admin.
    pub params: ContractStateParams,
}

/// Contract parameters that could be adjusted in a transaction by the contract admin.
#[cw_serde]
pub struct ContractStateParams {
    /// Parameters to do with delegations.
    pub delegations_params: DelegationsParams,

    /// Parameters to do with node operators.
    pub operators_params: OperatorsParams,

    /// Parameters to do with the config score
    pub config_score_params: ConfigScoreParams,
}

#[cw_serde]
pub struct ContractStateParamsUpdate {
    pub delegations_params: Option<DelegationsParams>,
    pub operators_params: Option<OperatorsParamsUpdate>,
    pub config_score_params: Option<ConfigScoreParamsUpdate>,
}

impl ContractStateParamsUpdate {
    pub fn contains_updates(&self) -> bool {
        self.delegations_params.is_some()
            || self.operators_params.is_some()
            || self.config_score_params.is_some()
    }
}

#[cw_serde]
pub struct DelegationsParams {
    /// Minimum amount a delegator must stake in orders for his delegation to get accepted.
    pub minimum_delegation: Option<Coin>,
}

#[cw_serde]
pub struct OperatorsParams {
    /// Minimum amount a node must pledge to get into the system.
    pub minimum_pledge: Coin,

    /// Defines the allowed profit margin range of operators.
    /// default: 0% - 100%
    pub profit_margin: ProfitMarginRange,

    /// Defines the allowed interval operating cost range of operators.
    /// default: 0 - 1'000'000'000'000'000 (1 Billion native tokens - the total supply)
    pub interval_operating_cost: OperatingCostRange,
}

#[cw_serde]
pub struct OperatorsParamsUpdate {
    pub minimum_pledge: Option<Coin>,
    pub profit_margin: Option<ProfitMarginRange>,
    pub interval_operating_cost: Option<OperatingCostRange>,
}

impl OperatorsParamsUpdate {
    pub fn contains_updates(&self) -> bool {
        self.minimum_pledge.is_some()
            || self.profit_margin.is_some()
            || self.interval_operating_cost.is_some()
    }
}

#[cw_serde]
pub struct ConfigScoreParamsUpdate {
    pub version_weights: Option<OutdatedVersionWeights>,
    pub version_score_formula_params: Option<VersionScoreFormulaParams>,
}

impl ConfigScoreParamsUpdate {
    pub fn contains_updates(&self) -> bool {
        self.version_weights.is_some() || self.version_score_formula_params.is_some()
    }
}
