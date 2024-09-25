// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::helpers::IntoBaseDecimal;
use crate::nym_node::Role;
use crate::{error::MixnetContractError, Percent};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

pub type Performance = Percent;
pub type WorkFactor = Decimal;

/// Parameters required by the mix-mining reward distribution that do not change during an interval.
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/IntervalRewardParams.ts"
    )
)]
#[cw_serde]
#[derive(Copy)]
pub struct IntervalRewardParams {
    /// Current value of the rewarding pool.
    /// It is expected to be constant throughout the interval.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub reward_pool: Decimal,

    /// Current value of the staking supply.
    /// It is expected to be constant throughout the interval.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub staking_supply: Decimal,

    /// Defines the percentage of stake needed to reach saturation for all of the nodes in the rewarded set.
    /// Also known as `beta`.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub staking_supply_scale_factor: Percent,

    // computed values
    /// Current value of the computed reward budget per epoch, per node.
    /// It is expected to be constant throughout the interval.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub epoch_reward_budget: Decimal,

    /// Current value of the stake saturation point.
    /// It is expected to be constant throughout the interval.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub stake_saturation_point: Decimal,

    // constants(-ish)
    // default: 30%
    /// Current value of the sybil resistance percent (`alpha`).
    /// It is not really expected to be changing very often.
    /// As a matter of fact, unless there's a very specific reason, it should remain constant.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub sybil_resistance: Percent,

    // default: 10
    /// Current active set work factor.
    /// It is not really expected to be changing very often.
    /// As a matter of fact, unless there's a very specific reason, it should remain constant.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub active_set_work_factor: Decimal,

    // default: 2%
    /// Current maximum interval pool emission.
    /// Assuming all nodes in the rewarded set are fully saturated and have 100% performance,
    /// this % of the reward pool would get distributed in rewards to all operators and its delegators.
    /// It is not really expected to be changing very often.
    /// As a matter of fact, unless there's a very specific reason, it should remain constant.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub interval_pool_emission: Percent,
}

impl IntervalRewardParams {
    pub fn to_inline_json(&self) -> String {
        serde_json_wasm::to_string(self).unwrap_or_else(|_| "serialisation failure".into())
    }
}

/// Parameters used for reward calculation.
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/RewardingParams.ts"
    )
)]
#[cw_serde]
#[derive(Copy)]
pub struct RewardingParams {
    /// Parameters that should remain unchanged throughout an interval.
    pub interval: IntervalRewardParams,

    pub rewarded_set: RewardedSetParams,
}

impl RewardingParams {
    pub fn active_node_work(&self) -> WorkFactor {
        self.interval.active_set_work_factor * self.standby_node_work()
    }

    pub fn standby_node_work(&self) -> WorkFactor {
        let f = self.interval.active_set_work_factor;
        let k = self.dec_rewarded_set_size();
        let one = Decimal::one();

        // nodes in reserve
        let k_r = self.dec_standby_set_size();

        one / (f * k - (f - one) * k_r)
    }

    pub fn rewarded_set_size(&self) -> u32 {
        self.rewarded_set.rewarded_set_size()
    }

    pub fn active_set_size(&self) -> u32 {
        self.rewarded_set.active_set_size()
    }

    pub fn dec_rewarded_set_size(&self) -> Decimal {
        // the unwrap here is fine as we're guaranteed an `u32` is going to fit in a Decimal
        // with 0 decimal places
        #[allow(clippy::unwrap_used)]
        self.rewarded_set_size().into_base_decimal().unwrap()
    }

    pub fn dec_active_set_size(&self) -> Decimal {
        // the unwrap here is fine as we're guaranteed an `u32` is going to fit in a Decimal
        // with 0 decimal places
        #[allow(clippy::unwrap_used)]
        self.active_set_size().into_base_decimal().unwrap()
    }

    fn dec_standby_set_size(&self) -> Decimal {
        // the unwrap here is fine as we're guaranteed an `u32` is going to fit in a Decimal
        // with 0 decimal places
        #[allow(clippy::unwrap_used)]
        self.rewarded_set.standby.into_base_decimal().unwrap()
    }

    pub fn apply_epochs_in_interval_change(&mut self, new_epochs_in_interval: u32) {
        // the unwrap here is fine as we're guaranteed an `u32` is going to fit in a Decimal
        // with 0 decimal places
        #[allow(clippy::unwrap_used)]
        let new_epochs_in_interval = new_epochs_in_interval.into_base_decimal().unwrap();

        self.interval.epoch_reward_budget = self.interval.reward_pool / new_epochs_in_interval
            * self.interval.interval_pool_emission;
    }

    pub fn validate_active_set_update(
        &self,
        update: ActiveSetUpdate,
    ) -> Result<(), MixnetContractError> {
        update.ensure_non_empty()?;
        let active_set_size = update.active_set_size();

        if active_set_size > self.rewarded_set_size() {
            return Err(MixnetContractError::InvalidActiveSetSize);
        }

        Ok(())
    }

    pub fn try_change_active_set(
        &mut self,
        update: ActiveSetUpdate,
    ) -> Result<(), MixnetContractError> {
        self.validate_active_set_update(update)?;
        let active_set_size = update.active_set_size();
        let rewarded_set_size = self.rewarded_set_size();

        // safety: due to validation we know that the active_set_size <= rewarded_set_size
        let new_standby = rewarded_set_size - active_set_size;

        self.rewarded_set.exit_gateways = update.exit_gateways;
        self.rewarded_set.entry_gateways = update.entry_gateways;
        self.rewarded_set.mixnodes = update.mixnodes;
        self.rewarded_set.standby = new_standby;
        Ok(())
    }

    pub fn try_apply_updates(
        &mut self,
        updates: IntervalRewardingParamsUpdate,
        epochs_in_interval: u32,
    ) -> Result<(), MixnetContractError> {
        if !updates.contains_updates() {
            return Err(MixnetContractError::EmptyParamsChangeMsg);
        }

        let mut recompute_epoch_budget = false;
        let mut recompute_saturation_point = false;

        if let Some(reward_pool) = updates.reward_pool {
            recompute_epoch_budget = true;
            self.interval.reward_pool = reward_pool;
        }

        if let Some(staking_supply) = updates.staking_supply {
            recompute_saturation_point = true;
            self.interval.staking_supply = staking_supply;
        }

        if let Some(staking_supply_scale_factor) = updates.staking_supply_scale_factor {
            self.interval.staking_supply_scale_factor = staking_supply_scale_factor
        }

        if let Some(sybil_resistance_percent) = updates.sybil_resistance_percent {
            self.interval.sybil_resistance = sybil_resistance_percent;
        }

        if let Some(active_set_work_factor) = updates.active_set_work_factor {
            self.interval.active_set_work_factor = active_set_work_factor;
        }

        if let Some(interval_pool_emission) = updates.interval_pool_emission {
            recompute_epoch_budget = true;
            self.interval.interval_pool_emission = interval_pool_emission;
        }

        if let Some(rewarded_set_update) = updates.rewarded_set_params {
            rewarded_set_update.ensure_valid()?;
            recompute_saturation_point = true;
            self.rewarded_set = rewarded_set_update;
        }

        if recompute_epoch_budget {
            self.interval.epoch_reward_budget = self.interval.reward_pool
                / epochs_in_interval.into_base_decimal()?
                * self.interval.interval_pool_emission;
        }

        if recompute_saturation_point {
            self.interval.stake_saturation_point =
                self.interval.staking_supply / self.rewarded_set_size().into_base_decimal()?
        }

        Ok(())
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/RewardedSetParams.ts"
    )
)]
#[cw_serde]
#[derive(Copy)]
pub struct RewardedSetParams {
    /// The expected number of nodes assigned entry gateway role (i.e. [`Role::EntryGateway`])
    pub entry_gateways: u32,

    /// The expected number of nodes assigned exit gateway role (i.e. [`Role::ExitGateway`])
    pub exit_gateways: u32,

    /// The expected number of nodes assigned the 'mixnode' role, i.e. total of [`Role::Layer1`], [`Role::Layer2`] and [`Role::Layer3`].
    pub mixnodes: u32,

    /// Number of nodes in the 'standby' set. (i.e. [`Role::Standby`])
    pub standby: u32,
}

impl RewardedSetParams {
    pub fn active_set_size(&self) -> u32 {
        self.entry_gateways + self.exit_gateways + self.mixnodes
    }

    pub fn rewarded_set_size(&self) -> u32 {
        self.active_set_size() + self.standby
    }

    pub fn ensure_valid(&self) -> Result<(), MixnetContractError> {
        if self.entry_gateways == 0 || self.exit_gateways == 0 || self.mixnodes == 0 {
            return Err(MixnetContractError::EmptyRoleAssignment);
        }
        if self.mixnodes % 3 != 0 {
            return Err(MixnetContractError::UnevenLayerAssignment);
        }
        Ok(())
    }

    pub fn maximum_role_count(&self, role: Role) -> u32 {
        match role {
            Role::EntryGateway => self.entry_gateways,
            Role::Layer1 | Role::Layer2 | Role::Layer3 => self.mixnodes / 3,
            Role::ExitGateway => self.exit_gateways,
            Role::Standby => self.standby,
        }
    }

    pub fn ensure_role_count(&self, role: Role, assigned: u32) -> Result<(), MixnetContractError> {
        let allowed = self.maximum_role_count(role);

        if assigned > allowed {
            return Err(MixnetContractError::IllegalRoleCount {
                role,
                assigned,
                allowed,
            });
        }

        Ok(())
    }
}

/// Parameters used for rewarding particular node.
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/NodeRewardingParameters.ts"
    )
)]
#[cw_serde]
#[derive(Copy)]
pub struct NodeRewardingParameters {
    /// Performance of the particular node in the current epoch.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub performance: Performance,

    /// Amount of work performed by this node in the current epoch
    /// also known as 'omega' in the paper
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub work_factor: WorkFactor,
}

impl NodeRewardingParameters {
    pub fn new(performance: Performance, work_factor: WorkFactor) -> Self {
        NodeRewardingParameters {
            performance,
            work_factor,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.performance.is_zero() || self.work_factor.is_zero()
    }
}

/// Specification on how the rewarding params should be updated.
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/IntervalRewardingParamsUpdate.ts"
    )
)]
#[cw_serde]
#[derive(Copy, Default)]
pub struct IntervalRewardingParamsUpdate {
    #[cfg_attr(feature = "generate-ts", ts(type = "string | null"))]
    /// Defines the new value of the reward pool.
    pub reward_pool: Option<Decimal>,

    #[cfg_attr(feature = "generate-ts", ts(type = "string | null"))]
    /// Defines the new value of the staking supply.
    pub staking_supply: Option<Decimal>,

    #[cfg_attr(feature = "generate-ts", ts(type = "string | null"))]
    /// Defines the new value of the staking supply scale factor.
    pub staking_supply_scale_factor: Option<Percent>,

    #[cfg_attr(feature = "generate-ts", ts(type = "string | null"))]
    /// Defines the new value of the sybil resistance percent.
    pub sybil_resistance_percent: Option<Percent>,

    #[cfg_attr(feature = "generate-ts", ts(type = "string | null"))]
    /// Defines the new value of the active set work factor.
    pub active_set_work_factor: Option<Decimal>,

    #[cfg_attr(feature = "generate-ts", ts(type = "string | null"))]
    /// Defines the new value of the interval pool emission rate.
    pub interval_pool_emission: Option<Percent>,

    /// Defines the parameters of the rewarded set.
    pub rewarded_set_params: Option<RewardedSetParams>,
}

impl IntervalRewardingParamsUpdate {
    pub fn contains_updates(&self) -> bool {
        // essentially at least a single field has to be a `Some`
        self.reward_pool.is_some()
            || self.staking_supply.is_some()
            || self.staking_supply_scale_factor.is_some()
            || self.sybil_resistance_percent.is_some()
            || self.active_set_work_factor.is_some()
            || self.interval_pool_emission.is_some()
            || self.rewarded_set_params.is_some()
    }

    pub fn to_inline_json(&self) -> String {
        serde_json_wasm::to_string(self).unwrap_or_else(|_| "serialisation failure".into())
    }
}

/// Specification on how the active set should be updated.
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/ActiveSetUpdate.ts"
    )
)]
#[cw_serde]
#[derive(Copy, Default)]
pub struct ActiveSetUpdate {
    /// The expected number of nodes assigned entry gateway role (i.e. [`Role::EntryGateway`])
    pub entry_gateways: u32,

    /// The expected number of nodes assigned exit gateway role (i.e. [`Role::ExitGateway`])
    pub exit_gateways: u32,

    /// The expected number of nodes assigned the 'mixnode' role, i.e. total of [`Role::Layer1`], [`Role::Layer2`] and [`Role::Layer3`].
    pub mixnodes: u32,
}

impl ActiveSetUpdate {
    pub fn active_set_size(&self) -> u32 {
        self.entry_gateways + self.exit_gateways + self.mixnodes
    }

    pub fn ensure_non_empty(&self) -> Result<(), MixnetContractError> {
        if self.entry_gateways == 0 || self.exit_gateways == 0 || self.mixnodes == 0 {
            return Err(MixnetContractError::EmptyRoleAssignment);
        }
        Ok(())
    }
}
