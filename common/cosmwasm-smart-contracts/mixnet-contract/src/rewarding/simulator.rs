// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::UNIT_DELEGATION_BASE;
use crate::error::MixnetContractError;
use crate::mixnode::{MixNodeCostParams, MixNodeRewarding};
use crate::reward_params::{IntervalRewardParams, NodeRewardParams, RewardingParams};
use crate::rewarding::helpers::truncate_reward;
use crate::rewarding::RewardDistribution;
use crate::{Delegation, Interval, Percent};
use cosmwasm_std::{Addr, Coin, Decimal};

pub struct Simulator {
    pub node_rewarding_details: MixNodeRewarding,
    pub node_delegations: Vec<Delegation>,
    pub system_rewarding_params: RewardingParams,

    pub interval: Interval,
}

impl Simulator {
    pub fn new(
        profit_margin_percent: Percent,
        interval_operating_cost: Coin,
        system_rewarding_params: RewardingParams,
        initial_pledge: Coin,
        interval: Interval,
    ) -> Self {
        let cost_params = MixNodeCostParams {
            profit_margin_percent,
            interval_operating_cost,
        };

        Simulator {
            node_rewarding_details: MixNodeRewarding::initialise_new(
                cost_params,
                &initial_pledge,
                Default::default(),
            ),
            // node_historical_records: [(0, HistoricalRewards::new_zeroth())].into_iter().collect(),
            node_delegations: vec![],
            system_rewarding_params,
            interval,
        }
    }

    pub fn delegate(&mut self, amount: Coin) {
        let cumulative_reward_ratio = self.node_rewarding_details.total_unit_reward;
        // let record = self.node_rewarding_details.increment_period();
        // self.node_historical_records.insert(period, record);
        self.node_rewarding_details
            .add_base_delegation(amount.amount);
        self.node_rewarding_details.unique_delegations += 1;

        // we don't care about the owner/node details here
        self.node_delegations.push(Delegation::new(
            Addr::unchecked("bob"),
            42,
            cumulative_reward_ratio,
            amount,
            123,
            None,
        ))
    }

    // TODO: ending period and all that stuff, to optimise it later
    pub fn determine_delegation_reward(&self, delegation: &Delegation) -> Decimal {
        self.node_rewarding_details
            .determine_delegation_reward(delegation)

        // // let starting_entry = self
        // //     .node_historical_records
        // //     .get(&delegation.period)
        // //     .expect("the delegation has been incorrectly saved");
        // //
        // // let starting_ratio = starting_entry.cumulative_reward_ratio;
        // // let ending_ratio = self.node_rewarding_details.full_reward_ratio();
        // // let adjust = starting_entry.cumulative_reward_ratio + UNIT_DELEGATION_BASE;
        // //
        // // (ending_ratio - starting_ratio) * delegation.dec_amount() / adjust
        //
        // let starting_ratio = delegation.cumulative_reward_ratio;
        // let ending_ratio = self.node_rewarding_details.full_reward_ratio();
        // let adjust = starting_ratio + UNIT_DELEGATION_BASE;
        //
        // (ending_ratio - starting_ratio) * delegation.dec_amount() / adjust
    }

    // since this is a simulator only, not something to be used in the production code, the unwraps are fine
    // if user inputs are invalid
    pub fn undelegate(
        &mut self,
        delegation_index: usize,
    ) -> Result<(Coin, Coin), MixnetContractError> {
        let delegation = self.node_delegations.remove(delegation_index);
        let reward = self.determine_delegation_reward(&delegation);
        self.node_rewarding_details
            .decrease_delegates(delegation.dec_amount() + reward)?;
        self.node_rewarding_details.unique_delegations -= 1;

        let reward_denom = &delegation.amount.denom;
        let truncated_reward = truncate_reward(reward, reward_denom);

        // if this was last delegation, move all leftover decimal tokens to the operator
        // (this is literally in the order of a millionth of a micronym)
        if self.node_delegations.is_empty() {
            assert_eq!(self.node_rewarding_details.unique_delegations, 0);
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

    pub fn simulate_epoch(&mut self, node_params: NodeRewardParams) -> RewardDistribution {
        let reward_distribution = self.node_rewarding_details.calculate_epoch_reward(
            &self.system_rewarding_params,
            node_params,
            self.interval.epochs_in_interval(),
        );
        self.node_rewarding_details
            .distribute_rewards(reward_distribution, self.interval.current_full_epoch_id());
        self.interval = self.interval.advance_epoch();

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

    // assume node state doesn't change in the interval (kinda unrealistic)
    pub fn simulate_interval(&mut self, node_params: NodeRewardParams) {
        assert_eq!(self.interval.current_epoch_id(), 0);
        let id = self.interval.current_interval_id();
        let mut distributed = Decimal::zero();
        for _ in 0..self.interval.epochs_in_interval() {
            let distr = self.simulate_epoch(node_params);
            distributed += distr.operator;
            distributed += distr.delegates;
        }
        assert_eq!(id + 1, self.interval.current_interval_id());

        // update reward pool and all of that
        let old = self.system_rewarding_params.interval;
        let reward_pool = old.reward_pool - distributed;
        let staking_supply = old.staking_supply + distributed;
        let epoch_reward_budget = reward_pool
            / Decimal::from_atomics(self.interval.epochs_in_interval(), 0).unwrap()
            * old.interval_pool_emission.value();
        let stake_saturation_point = staking_supply
            / Decimal::from_atomics(self.system_rewarding_params.rewarded_set_size, 0).unwrap();

        let updated_params = RewardingParams {
            interval: IntervalRewardParams {
                reward_pool,
                staking_supply,
                epoch_reward_budget,
                stake_saturation_point,
                sybil_resistance: old.sybil_resistance,
                active_set_work_factor: old.active_set_work_factor,
                interval_pool_emission: old.interval_pool_emission,
            },
            rewarded_set_size: self.system_rewarding_params.rewarded_set_size,
            active_set_size: self.system_rewarding_params.active_set_size,
        };

        self.system_rewarding_params = updated_params;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reward_params::IntervalRewardParams;
    use crate::rewarding::helpers::truncate_reward_amount;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{coin, Uint128};
    use std::time::Duration;

    fn base_simulator(initial_pledge: u128) -> Simulator {
        let profit_margin = Percent::from_percentage_value(10).unwrap();
        let interval_operating_cost = Coin::new(40_000_000, "unym");
        let epochs_in_interval = 720u32;
        let rewarded_set_size = 240;
        let active_set_size = 100;
        let interval_pool_emission = Percent::from_percentage_value(2).unwrap();

        let reward_pool = 250_000_000_000_000u128;
        let staking_supply = 100_000_000_000_000u128;
        let epoch_reward_budget =
            interval_pool_emission * Decimal::from_ratio(reward_pool, epochs_in_interval);
        let stake_saturation_point = Decimal::from_ratio(staking_supply, rewarded_set_size);

        let rewarding_params = RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: Decimal::from_atomics(reward_pool, 0).unwrap(), // 250M * 1M (we're expressing it all in base tokens)
                staking_supply: Decimal::from_atomics(staking_supply, 0).unwrap(), // 100M * 1M
                epoch_reward_budget,
                stake_saturation_point,
                sybil_resistance: Percent::from_percentage_value(30).unwrap(),
                active_set_work_factor: Decimal::percent(1000), // value '10'
                interval_pool_emission,
            },
            rewarded_set_size,
            active_set_size,
        };

        let interval = Interval::init_interval(
            epochs_in_interval,
            Duration::from_secs(60 * 60),
            &mock_env(),
        );
        let initial_pledge = Coin::new(initial_pledge, "unym");
        Simulator::new(
            profit_margin,
            interval_operating_cost,
            rewarding_params,
            initial_pledge,
            interval,
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
            NodeRewardParams::new(Percent::from_percentage_value(100).unwrap(), true);
        let rewards = simulator.simulate_epoch(epoch_params);

        assert_eq!(rewards.delegates, Decimal::zero());
        compare_decimals(rewards.operator, "1128452.5416104363".parse().unwrap());
    }

    #[test]
    fn single_delegation_at_genesis() {
        let mut simulator = base_simulator(10000_000000);
        simulator.delegate(Coin::new(18000_000000, "unym"));

        let node_params = NodeRewardParams::new(Percent::from_percentage_value(100).unwrap(), true);
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
        let node_params = NodeRewardParams::new(Percent::from_percentage_value(100).unwrap(), true);

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
    fn withdrawing_operator_reward() {
        // essentially all delegators' rewards (and the operator itself) are still correctly computed
        let original_pledge = coin(10000_000000, "unym");
        let mut simulator = base_simulator(original_pledge.amount.u128());
        let node_params = NodeRewardParams::new(Percent::from_percentage_value(100).unwrap(), true);

        // add 2 delegations at genesis (because it makes things easier and as shown with previous tests
        // delegating at different times still work)
        simulator.delegate(Coin::new(18000_000000, "unym"));
        simulator.delegate(Coin::new(4000_000000, "unym"));

        // "normal", sanity check rewarding
        let rewards1 = simulator.simulate_epoch(node_params);
        let expected_operator1 = "1411087.1007647323".parse().unwrap();
        let expected_delegator_reward1 = "2199961.032388664".parse().unwrap();
        compare_decimals(rewards1.delegates, expected_delegator_reward1);
        compare_decimals(rewards1.operator, expected_operator1);
        check_rewarding_invariant(&simulator);

        let reward = simulator
            .node_rewarding_details
            .withdraw_operator_reward(&original_pledge);
        assert_eq!(reward.amount, truncate_reward_amount(expected_operator1));
        assert_eq!(
            simulator.node_rewarding_details.operator,
            Decimal::from_atomics(original_pledge.amount, 0).unwrap()
        );

        let rewards2 = simulator.simulate_epoch(node_params);
        let expected_operator2 = "1411113.0004067947".parse().unwrap();
        let expected_delegator_reward2 = "2200183.3879084454".parse().unwrap();
        compare_decimals(rewards2.delegates, expected_delegator_reward2);
        compare_decimals(rewards2.operator, expected_operator2);
        check_rewarding_invariant(&simulator);
    }

    #[test]
    fn withdrawing_delegator_reward() {
        // essentially all delegators' rewards (and the operator itself) are still correctly computed
        let mut simulator = base_simulator(10000_000000);
        let node_params = NodeRewardParams::new(Percent::from_percentage_value(100).unwrap(), true);

        // add 2 delegations at genesis (because it makes things easier and as shown with previous tests
        // delegating at different times still work)
        simulator.delegate(Coin::new(18000_000000, "unym"));
        simulator.delegate(Coin::new(4000_000000, "unym"));

        // "normal", sanity check rewarding
        let rewards1 = simulator.simulate_epoch(node_params);
        let expected_operator1 = "1411087.1007647323".parse().unwrap();
        let expected_delegator_reward1 = "2199961.032388664".parse().unwrap();
        compare_decimals(rewards1.delegates, expected_delegator_reward1);
        compare_decimals(rewards1.operator, expected_operator1);
        check_rewarding_invariant(&simulator);

        // reference to our `18000_000000` delegation
        let delegation1 = &mut simulator.node_delegations[0];
        let reward = simulator
            .node_rewarding_details
            .withdraw_delegator_reward(delegation1)
            .unwrap();
        let expected_del1_reward = "1799968.1174089068".parse().unwrap();
        assert_eq!(reward.amount, truncate_reward_amount(expected_del1_reward));

        // new reward after withdrawal
        let rewards2 = simulator.simulate_epoch(node_params);
        let expected_operator2 = "1411250.1907492676".parse().unwrap();
        let expected_delegator_reward2 = "2200004.051009689".parse().unwrap();
        compare_decimals(rewards2.delegates, expected_delegator_reward2);
        compare_decimals(rewards2.operator, expected_operator2);
        check_rewarding_invariant(&simulator);

        // check final values
        let reward_del1 = simulator
            .node_rewarding_details
            .withdraw_delegator_reward(&mut simulator.node_delegations[0])
            .unwrap();
        let expected_del1_reward = "1799970.5883041779".parse().unwrap();
        assert_eq!(
            reward_del1.amount,
            truncate_reward_amount(expected_del1_reward)
        );

        let reward_del2 = simulator
            .node_rewarding_details
            .withdraw_delegator_reward(&mut simulator.node_delegations[1])
            .unwrap();
        let first: Decimal = "399992.91497975704".parse().unwrap();
        let second: Decimal = "400033.4627055114".parse().unwrap();
        let expected_del2_reward = first + second;
        assert_eq!(
            reward_del2.amount,
            truncate_reward_amount(expected_del2_reward)
        );
    }

    #[test]
    fn simulating_multiple_epochs() {
        let mut simulator = base_simulator(10000_000000);

        let mut is_active = true;
        let mut performance = Percent::from_percentage_value(100).unwrap();
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
                performance = Percent::from_percentage_value(90).unwrap();
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
                performance = Percent::from_percentage_value(100).unwrap();
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
