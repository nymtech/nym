// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::UNIT_DELEGATION_BASE;
use crate::error::MixnetContractError;
use crate::mixnode::{MixNodeCostParams, MixNodeRewarding, Period};
use crate::reward_params::{NodeRewardParams, RewardingParams};
use crate::rewarding::helpers::truncate_reward;
use crate::rewarding::{HistoricalRewards, RewardDistribution};
use crate::{Delegation, Percent};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use std::collections::HashMap;

pub struct Simulator {
    pub node_rewarding_details: MixNodeRewarding,

    // note that delegations and historical data are stored under separate storage keys in the contract
    pub node_historical_records: HashMap<Period, HistoricalRewards>,
    pub node_delegations: Vec<Delegation>,
    pub system_rewarding_params: RewardingParams,

    epoch_id: u32,
}

impl Simulator {
    pub fn new(
        profit_margin_percent: Percent,
        interval_operating_cost: Coin,
        system_rewarding_params: RewardingParams,
        initial_pledge: Coin,
    ) -> Self {
        let cost_params = MixNodeCostParams {
            profit_margin_percent,
            interval_operating_cost,
        };

        Simulator {
            node_rewarding_details: MixNodeRewarding::initialise_new(
                cost_params,
                &initial_pledge,
                0,
            ),
            node_historical_records: [(0, HistoricalRewards::new_zeroth())].into_iter().collect(),
            node_delegations: vec![],
            system_rewarding_params,
            epoch_id: 0,
        }
    }

    pub fn delegate(&mut self, amount: Coin) {
        let period = self.node_rewarding_details.current_period;
        let record = self.node_rewarding_details.increment_period();
        self.node_historical_records.insert(period, record);
        self.node_rewarding_details.add_base_delegation(&amount);

        // we don't care about the owner/node details here
        self.node_delegations.push(Delegation::new(
            Addr::unchecked("bob"),
            42,
            period,
            amount,
            123,
            None,
        ))
    }

    // TODO: ending period and all that stuff, to optimise it later
    pub fn determine_delegation_reward(&self, delegation: &Delegation) -> Decimal {
        let starting_entry = self
            .node_historical_records
            .get(&delegation.period)
            .expect("the delegation has been incorrectly saved");

        let starting_ratio = starting_entry.cumulative_reward_ratio;
        let ending_ratio = self.node_rewarding_details.full_reward_ratio();
        let adjust = starting_entry.cumulative_reward_ratio + UNIT_DELEGATION_BASE;

        (ending_ratio - starting_ratio) * delegation.dec_amount() / adjust
    }

    fn decrement_historical_ref_count_or_remove(&mut self, period: Period) {
        let entry = self.node_historical_records.get_mut(&period).unwrap();
        if entry.reference_count == 1 {
            self.node_historical_records.remove(&period);
        } else {
            entry.reference_count -= 1;
        }
    }

    // since this is a simulator only, not something to be used in the production code, the unwraps are fine
    // if user inputs are invalid
    pub fn undelegate(
        &mut self,
        delegation_index: usize,
    ) -> Result<(Coin, Coin), MixnetContractError> {
        self.node_rewarding_details.increment_period();

        let delegation = self.node_delegations.remove(delegation_index);
        let reward = self.determine_delegation_reward(&delegation);
        self.decrement_historical_ref_count_or_remove(delegation.period);
        self.node_rewarding_details
            .decrease_delegates(delegation.dec_amount() + reward)?;

        let reward_denom = &delegation.amount.denom;
        let truncated_reward = truncate_reward(reward, reward_denom);

        // if this was last delegation, move all leftover decimal tokens to the operator
        // (this is literally in the order of a millionth of a micronym)
        if self.node_delegations.is_empty() {
            self.node_rewarding_details.operator += self.node_rewarding_details.delegates;
            self.node_rewarding_details.delegates = Decimal::zero();
        }

        Ok((delegation.amount, truncated_reward))
    }

    pub fn determine_total_delegation_reward(&self) -> Decimal {
        let mut total = Decimal::zero();
        for delegation in &self.node_delegations {
            total += self.determine_delegation_reward(delegation)
        }
        total
    }

    pub(crate) fn simulate_epoch(&mut self, node_params: NodeRewardParams) -> RewardDistribution {
        let reward_distribution = self
            .node_rewarding_details
            .calculate_epoch_reward(&self.system_rewarding_params, node_params);
        self.node_rewarding_details
            .distribute_rewards(reward_distribution, self.epoch_id)
            .unwrap();
        self.epoch_id += 1;

        // self.node_rewarding_details
        //     .epoch_rewarding(
        //         &self.system_rewarding_params,
        //         node_params,
        //         &self.node_cost_params,
        //         self.epoch_id,
        //     )
        //     .unwrap();

        reward_distribution
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reward_params::{EpochRewardParamsNew, IntervalRewardParams};

    fn base_simulator(initial_pledge: u128) -> Simulator {
        let profit_margin = Percent::from_percentage_value(10u32).unwrap();
        let interval_operating_cost = Coin::new(40_000_000, "unym");
        let epochs_in_interval = 720u32;
        let rewarded_set_size = 240;
        let active_set_size = 100;

        let reward_pool = 250_000_000_000_000u128;
        let staking_supply = 100_000_000_000_000u128;
        let epoch_reward_budget =
            Decimal::from_ratio(reward_pool, epochs_in_interval) * Decimal::percent(2);
        let stake_saturation_point = Decimal::from_ratio(staking_supply, rewarded_set_size);

        let rewarding_params = RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: Decimal::from_atomics(reward_pool, 0).unwrap(), // 250M * 1M (we're expressing it all in base tokens)
                staking_supply: Decimal::from_atomics(staking_supply, 0).unwrap(), // 100M * 1M
                epochs_in_interval,
                epoch_reward_budget,
                stake_saturation_point,
                sybil_resistance_percent: Decimal::percent(30),
                active_set_work_factor: Decimal::percent(1000), // value '10'
            },
            epoch: EpochRewardParamsNew {
                rewarded_set_size,
                active_set_size,
            },
        };

        let initial_pledge = Coin::new(initial_pledge, "unym");
        Simulator::new(
            profit_margin,
            interval_operating_cost,
            rewarding_params,
            initial_pledge,
        )
    }

    fn compare_decimals(a: Decimal, b: Decimal) {
        let epsilon = Decimal::from_ratio(1u128, 100_000_000u128);
        if a > b {
            assert!(a - b < epsilon, "{} != {}", a, b)
        } else {
            assert!(b - a < epsilon, "{} != {}", a, b)
        }
    }

    // essentially our delegations + estimated rewards HAVE TO equal to what we actually determined
    fn check_rewarding_invariant(simulator: &Simulator) {
        let delegation_sum: Decimal = simulator
            .node_delegations
            .iter()
            .map(|d| d.dec_amount())
            .sum();
        let reward_sum = simulator.determine_total_delegation_reward();
        compare_decimals(
            delegation_sum + reward_sum,
            simulator.node_rewarding_details.delegates,
        )
    }

    #[test]
    fn simulator_returns_expected_values_for_base_case() {
        let mut simulator = base_simulator(10000_000000);

        let epoch_params =
            NodeRewardParams::new(Percent::from_percentage_value(100u32).unwrap(), true);
        let rewards = simulator.simulate_epoch(epoch_params);

        assert_eq!(rewards.delegates, Decimal::zero());
        compare_decimals(rewards.operator, "1128452.5416104363".parse().unwrap());
    }

    #[test]
    fn single_delegation_at_genesis() {
        let mut simulator = base_simulator(10000_000000);
        simulator.delegate(Coin::new(18000_000000, "unym"));

        let node_params =
            NodeRewardParams::new(Percent::from_percentage_value(100u32).unwrap(), true);
        let rewards = simulator.simulate_epoch(node_params);

        compare_decimals(rewards.delegates, "1795950.2602660495".parse().unwrap());
        compare_decimals(rewards.operator, "1363716.856243172".parse().unwrap());

        compare_decimals(
            rewards.delegates,
            simulator.determine_total_delegation_reward(),
        );
        assert_eq!(
            simulator.node_rewarding_details.operator,
            rewards.operator + Decimal::from_atomics(10000_000000u128, 0).unwrap()
        );
        assert_eq!(
            simulator.node_rewarding_details.delegates,
            rewards.delegates + Decimal::from_atomics(18000_000000u128, 0).unwrap()
        );
    }

    #[test]
    fn delegation_and_undelegation() {
        let mut simulator = base_simulator(10000_000000);
        let node_params =
            NodeRewardParams::new(Percent::from_percentage_value(100u32).unwrap(), true);

        let rewards1 = simulator.simulate_epoch(node_params);
        let expected_operator1 = "1128452.5416104363".parse().unwrap();
        assert_eq!(rewards1.delegates, Decimal::zero());
        compare_decimals(rewards1.operator, expected_operator1);

        simulator.delegate(Coin::new(18000_000000, "unym"));

        let rewards2 = simulator.simulate_epoch(node_params);
        let expected_operator2 = "1363843.413584609".parse().unwrap();
        let expected_delegator_reward1 = "1795952.25874404".parse().unwrap();
        compare_decimals(rewards2.delegates, expected_delegator_reward1);
        compare_decimals(rewards2.operator, expected_operator2);

        let rewards3 = simulator.simulate_epoch(node_params);
        let expected_operator3 = "1364017.7824440491".parse().unwrap();
        let expected_delegator_reward2 = "1796135.9269468693".parse().unwrap();
        compare_decimals(rewards3.delegates, expected_delegator_reward2);
        compare_decimals(rewards3.operator, expected_operator3);

        let (delegation, reward) = simulator.undelegate(0).unwrap();
        assert_eq!(delegation.amount.u128(), 18000_000000);
        assert_eq!(
            reward.amount,
            (expected_delegator_reward1 + expected_delegator_reward2) * Uint128::new(1)
        );

        let base_op = Decimal::from_atomics(10000_000000u128, 0).unwrap();
        compare_decimals(
            simulator.node_rewarding_details.operator,
            base_op + expected_operator1 + expected_operator2 + expected_operator3,
        );
        assert_eq!(Decimal::zero(), simulator.node_rewarding_details.delegates);
    }

    #[test]
    fn simulating_multiple_epochs() {
        let mut simulator = base_simulator(10000_000000);

        let mut is_active = true;
        let mut performance = Percent::from_percentage_value(100u32).unwrap();
        for epoch in 0..720 {
            if epoch == 0 {
                simulator.delegate(Coin::new(18000_000000, "unym"))
            }
            if epoch == 42 {
                simulator.delegate(Coin::new(2000_000000, "unym"))
            }
            if epoch == 89 {
                is_active = false;
            }
            if epoch == 123 {
                simulator.delegate(Coin::new(6666_000000, "unym"))
            }
            if epoch == 167 {
                performance = Percent::from_percentage_value(90u32).unwrap();
            }
            if epoch == 245 {
                simulator.delegate(Coin::new(2050_000000, "unym"))
            }
            if epoch == 264 {
                let (delegation, _reward) = simulator.undelegate(1).unwrap();
                // sanity check to make sure we undelegated what we wanted to undelegate : )
                assert_eq!(delegation.amount.u128(), 2000_000000);
                // TODO: figure out if there's a good way to verify whether `reward` is what we expect it to be
            }
            if epoch == 345 {
                is_active = true;
            }
            if epoch == 358 {
                performance = Percent::from_percentage_value(100u32).unwrap();
            }
            if epoch == 458 {
                let (delegation, _reward) = simulator.undelegate(0).unwrap();
                // sanity check to make sure we undelegated what we wanted to undelegate : )
                assert_eq!(delegation.amount.u128(), 18000_000000);
                // TODO: figure out if there's a good way to verify whether `reward` is what we expect it to be
            }
            if epoch == 545 {
                simulator.delegate(Coin::new(5000_000000, "unym"))
            }

            // this has to always hold
            check_rewarding_invariant(&simulator);
            let node_params = NodeRewardParams::new(performance, is_active);
            simulator.simulate_epoch(node_params);
        }

        // after everyone undelegates, there should be nothing left in the delegates pool
        let delegations = simulator.node_delegations.len();
        for _ in 0..delegations {
            simulator.undelegate(0).unwrap();
        }
        assert_eq!(Decimal::zero(), simulator.node_rewarding_details.delegates);
    }
}
