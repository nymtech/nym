// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::helpers::IntoBaseDecimal;
use crate::reward_params::{NodeRewardingParameters, WorkFactor};
use crate::rewarding::simulator::simulated_node::SimulatedNode;
use crate::rewarding::RewardDistribution;
use crate::{Delegation, Interval, IntervalRewardParams, NodeCostParams, NodeId, RewardingParams};
use cosmwasm_std::{Coin, Decimal};
use std::collections::BTreeMap;

pub mod simulated_node;

pub struct Simulator {
    pub nodes: BTreeMap<NodeId, SimulatedNode>,
    pub system_rewarding_params: RewardingParams,
    pub interval: Interval,

    next_mix_id: NodeId,
    pending_reward_pool_emission: Decimal,
}

impl Simulator {
    pub fn new(system_rewarding_params: RewardingParams, interval: Interval) -> Self {
        Simulator {
            nodes: Default::default(),
            system_rewarding_params,
            interval,
            next_mix_id: 0,
            pending_reward_pool_emission: Default::default(),
        }
    }

    pub fn legacy_standby_work_factor(&self) -> WorkFactor {
        self.system_rewarding_params.standby_node_work()
    }

    pub fn legacy_active_work_factor(&self) -> WorkFactor {
        self.system_rewarding_params.active_node_work()
    }

    fn advance_epoch(&mut self) -> Result<(), MixnetContractError> {
        let updated = self.interval.advance_epoch();

        // we rolled over an interval
        if self.interval.current_interval_id() + 1 == updated.current_interval_id() {
            let old = self.system_rewarding_params.interval;
            let reward_pool = old.reward_pool - self.pending_reward_pool_emission;
            let staking_supply = old.staking_supply
                + self
                    .system_rewarding_params
                    .interval
                    .staking_supply_scale_factor
                    * self.pending_reward_pool_emission;
            let epoch_reward_budget = reward_pool
                / self.interval.epochs_in_interval().into_base_decimal()?
                * old.interval_pool_emission.value();
            let stake_saturation_point = staking_supply
                / self
                    .system_rewarding_params
                    .rewarded_set_size()
                    .into_base_decimal()?;

            let updated_params = RewardingParams {
                interval: IntervalRewardParams {
                    reward_pool,
                    staking_supply,
                    staking_supply_scale_factor: old.staking_supply_scale_factor,
                    epoch_reward_budget,
                    stake_saturation_point,
                    sybil_resistance: old.sybil_resistance,
                    active_set_work_factor: old.active_set_work_factor,
                    interval_pool_emission: old.interval_pool_emission,
                },
                rewarded_set: self.system_rewarding_params.rewarded_set,
            };

            self.system_rewarding_params = updated_params;
            self.pending_reward_pool_emission = Decimal::zero();
        }
        self.interval = updated;

        Ok(())
    }

    pub fn bond(
        &mut self,
        pledge: Coin,
        cost_params: NodeCostParams,
    ) -> Result<NodeId, MixnetContractError> {
        let mix_id = self.next_mix_id;

        self.nodes.insert(
            mix_id,
            SimulatedNode::new(
                mix_id,
                cost_params,
                &pledge,
                self.interval.current_epoch_absolute_id(),
            )?,
        );

        self.next_mix_id += 1;

        Ok(mix_id)
    }

    pub fn delegate<S: Into<String>>(
        &mut self,
        delegator: S,
        delegation: Coin,
        mix_id: NodeId,
    ) -> Result<(), MixnetContractError> {
        let node = self
            .nodes
            .get_mut(&mix_id)
            .ok_or(MixnetContractError::MixNodeBondNotFound { mix_id })?;
        node.delegate(delegator, delegation)
    }

    // since this is a simulator only, not something to be used in the production code, the unwraps are fine
    // if user inputs are invalid
    pub fn undelegate<S: Into<String>>(
        &mut self,
        delegator: S,
        mix_id: NodeId,
    ) -> Result<(Coin, Coin), MixnetContractError> {
        let node = self
            .nodes
            .get_mut(&mix_id)
            .ok_or(MixnetContractError::MixNodeBondNotFound { mix_id })?;
        node.undelegate(delegator)
    }

    pub fn simulate_epoch_single_node(
        &mut self,
        params: NodeRewardingParameters,
    ) -> Result<RewardDistribution, MixnetContractError> {
        assert_eq!(self.nodes.len(), 1);

        if let Some(&id) = self.nodes.keys().next() {
            let mut params_map = BTreeMap::new();
            params_map.insert(id, params);
            Ok(self
                .simulate_epoch(&params_map)?
                .remove(&id)
                .unwrap_or_default())
        } else {
            Ok(RewardDistribution::default())
        }
    }

    pub fn simulate_epoch(
        &mut self,
        node_params: &BTreeMap<NodeId, NodeRewardingParameters>,
    ) -> Result<BTreeMap<NodeId, RewardDistribution>, MixnetContractError> {
        let mut params_keys = node_params.keys().copied().collect::<Vec<_>>();
        params_keys.sort_unstable();
        let mut node_keys = self.nodes.keys().copied().collect::<Vec<_>>();
        node_keys.sort_unstable();

        if params_keys != node_keys {
            panic!("invalid node rewarding params provided");
        }

        let mut dist = BTreeMap::new();

        for (mix_id, node) in self.nodes.iter_mut() {
            let reward_distribution = node.rewarding_details.calculate_epoch_reward(
                &self.system_rewarding_params,
                node_params[mix_id],
                self.interval.epochs_in_interval(),
            );
            node.rewarding_details.distribute_rewards(
                reward_distribution,
                self.interval.current_epoch_absolute_id(),
            );
            self.pending_reward_pool_emission += reward_distribution.operator;
            self.pending_reward_pool_emission += reward_distribution.delegates;

            dist.insert(*mix_id, reward_distribution);
        }

        self.advance_epoch()?;
        Ok(dist)
    }

    pub fn determine_delegation_reward(
        &self,
        delegation: &Delegation,
    ) -> Result<Decimal, MixnetContractError> {
        Ok(self.nodes[&delegation.node_id]
            .rewarding_details
            .determine_delegation_reward(delegation)?)
    }

    pub fn determine_total_delegation_reward(&self) -> Result<Decimal, MixnetContractError> {
        let mut total = Decimal::zero();

        for node in self.nodes.values() {
            for delegation in node.delegations.values() {
                total += node
                    .rewarding_details
                    .determine_delegation_reward(delegation)?
            }
        }
        Ok(total)
    }

    // assume node state doesn't change in the interval (kinda unrealistic)
    pub fn simulate_full_interval(
        &mut self,
        node_params: &BTreeMap<NodeId, NodeRewardingParameters>,
    ) -> Result<(), MixnetContractError> {
        for _ in 0..self.interval.epochs_in_interval() {
            self.simulate_epoch(node_params)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::compare_decimals;
    use crate::reward_params::RewardedSetParams;
    use crate::Percent;
    use cosmwasm_std::testing::mock_env;
    use std::time::Duration;

    #[cfg(test)]
    mod single_node_case {
        use super::*;
        use crate::reward_params::RewardedSetParams;
        use crate::rewarding::helpers::truncate_reward_amount;
        use cosmwasm_std::coin;

        // explicitly marking this as part of #[allow(clippy::unwrap_used)] until
        // https://github.com/rust-lang/rust-clippy/pull/9686
        // is merged into a release
        #[allow(clippy::unwrap_used)]
        fn base_simulator(initial_pledge: u128) -> Simulator {
            let profit_margin = Percent::from_percentage_value(10).unwrap();
            let interval_operating_cost = Coin::new(40_000_000, "unym");
            let epochs_in_interval = 720u32;
            let interval_pool_emission = Percent::from_percentage_value(2).unwrap();

            // the import values here are active set being 100 and rewarded set being 240
            // since those are the values we were using in the past
            let rewarded_set = RewardedSetParams {
                entry_gateways: 20,
                exit_gateways: 50,
                mixnodes: 30,
                standby: 140,
            };

            let reward_pool = 250_000_000_000_000u128;
            let staking_supply = 100_000_000_000_000u128;
            let epoch_reward_budget =
                interval_pool_emission * Decimal::from_ratio(reward_pool, epochs_in_interval);
            let stake_saturation_point =
                Decimal::from_ratio(staking_supply, rewarded_set.rewarded_set_size());

            let rewarding_params = RewardingParams {
                interval: IntervalRewardParams {
                    reward_pool: Decimal::from_atomics(reward_pool, 0).unwrap(), // 250M * 1M (we're expressing it all in base tokens)
                    staking_supply: Decimal::from_atomics(staking_supply, 0).unwrap(), // 100M * 1M
                    staking_supply_scale_factor: Percent::hundred(),
                    epoch_reward_budget,
                    stake_saturation_point,
                    sybil_resistance: Percent::from_percentage_value(30).unwrap(),
                    active_set_work_factor: Decimal::percent(1000), // value '10'
                    interval_pool_emission,
                },
                rewarded_set,
            };

            let interval = Interval::init_interval(
                epochs_in_interval,
                Duration::from_secs(60 * 60),
                &mock_env(),
            );
            let initial_pledge = Coin::new(initial_pledge, "unym");
            let mut simulator = Simulator::new(rewarding_params, interval);

            let cost_params = NodeCostParams {
                profit_margin_percent: profit_margin,
                interval_operating_cost,
            };
            simulator.bond(initial_pledge, cost_params).unwrap();
            simulator
        }

        // essentially our delegations + estimated rewards HAVE TO equal to what we actually determined
        //
        // explicitly marking this as part of #[allow(clippy::unwrap_used)] until
        // https://github.com/rust-lang/rust-clippy/pull/9686
        // is merged into a release
        #[allow(clippy::unwrap_used)]
        fn check_rewarding_invariant(simulator: &Simulator) {
            for node in simulator.nodes.values() {
                let delegation_sum: Decimal = node
                    .delegations
                    .values()
                    .map(|d| d.dec_amount().unwrap())
                    .sum();

                let reward_sum: Decimal = node
                    .delegations
                    .values()
                    .map(|d| {
                        node.rewarding_details
                            .determine_delegation_reward(d)
                            .unwrap()
                    })
                    .sum();

                // let reward_sum = simulator.determine_total_delegation_reward();
                compare_decimals(
                    delegation_sum + reward_sum,
                    node.rewarding_details.delegates,
                    None,
                )
            }
        }

        #[test]
        fn simulator_returns_expected_values_for_base_case() {
            let mut simulator = base_simulator(10000_000000);

            let epoch_params = NodeRewardingParameters::new(
                Percent::from_percentage_value(100).unwrap(),
                simulator.legacy_active_work_factor(),
            );
            let rewards = simulator.simulate_epoch_single_node(epoch_params).unwrap();

            assert_eq!(rewards.delegates, Decimal::zero());
            compare_decimals(
                rewards.operator,
                "1128452.5416104363".parse().unwrap(),
                None,
            );
        }

        #[test]
        fn single_delegation_at_genesis() {
            let mut simulator = base_simulator(10000_000000);
            simulator
                .delegate("alice", Coin::new(18000_000000, "unym"), 0)
                .unwrap();

            let node_params = NodeRewardingParameters::new(
                Percent::from_percentage_value(100).unwrap(),
                simulator.legacy_active_work_factor(),
            );
            let rewards = simulator.simulate_epoch_single_node(node_params).unwrap();

            compare_decimals(
                rewards.delegates,
                "1795950.2602660495".parse().unwrap(),
                None,
            );
            compare_decimals(rewards.operator, "1363716.856243172".parse().unwrap(), None);

            compare_decimals(
                rewards.delegates,
                simulator.determine_total_delegation_reward().unwrap(),
                None,
            );
            let node = &simulator.nodes[&0];
            assert_eq!(
                node.rewarding_details.operator,
                rewards.operator + Decimal::from_atomics(10000_000000u128, 0).unwrap()
            );
            assert_eq!(
                node.rewarding_details.delegates,
                rewards.delegates + Decimal::from_atomics(18000_000000u128, 0).unwrap()
            );
        }

        #[test]
        fn delegation_and_undelegation() {
            let mut simulator = base_simulator(10000_000000);
            let node_params = NodeRewardingParameters::new(
                Percent::from_percentage_value(100).unwrap(),
                simulator.legacy_active_work_factor(),
            );

            let rewards1 = simulator.simulate_epoch_single_node(node_params).unwrap();
            let expected_operator1 = "1128452.5416104363".parse().unwrap();
            assert_eq!(rewards1.delegates, Decimal::zero());
            compare_decimals(rewards1.operator, expected_operator1, None);

            simulator
                .delegate("alice", Coin::new(18000_000000, "unym"), 0)
                .unwrap();

            let rewards2 = simulator.simulate_epoch_single_node(node_params).unwrap();
            let expected_operator2 = "1363843.413584609".parse().unwrap();
            let expected_delegator_reward1 = "1795952.25874404".parse().unwrap();
            compare_decimals(rewards2.delegates, expected_delegator_reward1, None);
            compare_decimals(rewards2.operator, expected_operator2, None);

            let rewards3 = simulator.simulate_epoch_single_node(node_params).unwrap();
            let expected_operator3 = "1364017.7824440491".parse().unwrap();
            let expected_delegator_reward2 = "1796135.9269468693".parse().unwrap();
            compare_decimals(rewards3.delegates, expected_delegator_reward2, None);
            compare_decimals(rewards3.operator, expected_operator3, None);

            let (delegation, reward) = simulator.undelegate("alice", 0).unwrap();
            assert_eq!(delegation.amount.u128(), 18000_000000);
            assert_eq!(
                reward.amount,
                truncate_reward_amount(expected_delegator_reward1 + expected_delegator_reward2)
            );

            let base_op = Decimal::from_atomics(10000_000000u128, 0).unwrap();

            let node = &simulator.nodes[&0];
            compare_decimals(
                node.rewarding_details.operator,
                base_op + expected_operator1 + expected_operator2 + expected_operator3,
                None,
            );
            assert_eq!(Decimal::zero(), node.rewarding_details.delegates);
        }

        #[test]
        fn withdrawing_operator_reward() {
            // essentially all delegators' rewards (and the operator itself) are still correctly computed
            let original_pledge = coin(10000_000000, "unym");
            let mut simulator = base_simulator(original_pledge.amount.u128());
            let node_params = NodeRewardingParameters::new(
                Percent::from_percentage_value(100).unwrap(),
                simulator.legacy_active_work_factor(),
            );

            // add 2 delegations at genesis (because it makes things easier and as shown with previous tests
            // delegating at different times still work)
            simulator
                .delegate("alice", Coin::new(18000_000000, "unym"), 0)
                .unwrap();
            simulator
                .delegate("bob", Coin::new(4000_000000, "unym"), 0)
                .unwrap();

            // "normal", sanity check rewarding
            let rewards1 = simulator.simulate_epoch_single_node(node_params).unwrap();
            let expected_operator1 = "1411087.1007647323".parse().unwrap();
            let expected_delegator_reward1 = "2199961.032388664".parse().unwrap();
            compare_decimals(rewards1.delegates, expected_delegator_reward1, None);
            compare_decimals(rewards1.operator, expected_operator1, None);
            check_rewarding_invariant(&simulator);

            let node = simulator.nodes.get_mut(&0).unwrap();
            let reward = node
                .rewarding_details
                .withdraw_operator_reward(&original_pledge)
                .unwrap();
            assert_eq!(reward.amount, truncate_reward_amount(expected_operator1));
            assert_eq!(
                node.rewarding_details.operator,
                Decimal::from_atomics(original_pledge.amount, 0).unwrap()
            );

            let rewards2 = simulator.simulate_epoch_single_node(node_params).unwrap();
            let expected_operator2 = "1411113.0004067947".parse().unwrap();
            let expected_delegator_reward2 = "2200183.3879084454".parse().unwrap();
            compare_decimals(rewards2.delegates, expected_delegator_reward2, None);
            compare_decimals(rewards2.operator, expected_operator2, None);
            check_rewarding_invariant(&simulator);
        }

        #[test]
        fn withdrawing_delegator_reward() {
            // essentially all delegators' rewards (and the operator itself) are still correctly computed
            let mut simulator = base_simulator(10000_000000);
            let node_params = NodeRewardingParameters::new(
                Percent::from_percentage_value(100).unwrap(),
                simulator.legacy_active_work_factor(),
            );

            // add 2 delegations at genesis (because it makes things easier and as shown with previous tests
            // delegating at different times still work)
            simulator
                .delegate("alice", Coin::new(18000_000000, "unym"), 0)
                .unwrap();
            simulator
                .delegate("bob", Coin::new(4000_000000, "unym"), 0)
                .unwrap();

            // "normal", sanity check rewarding
            let rewards1 = simulator.simulate_epoch_single_node(node_params).unwrap();
            let expected_operator1 = "1411087.1007647323".parse().unwrap();
            let expected_delegator_reward1 = "2199961.032388664".parse().unwrap();
            compare_decimals(rewards1.delegates, expected_delegator_reward1, None);
            compare_decimals(rewards1.operator, expected_operator1, None);
            check_rewarding_invariant(&simulator);

            // reference to our `18000_000000` delegation
            let node = simulator.nodes.get_mut(&0).unwrap();
            let delegation1 = node.delegations.get_mut("alice").unwrap();
            let reward = node
                .rewarding_details
                .withdraw_delegator_reward(delegation1)
                .unwrap();
            let expected_del1_reward = "1799968.1174089068".parse().unwrap();
            assert_eq!(reward.amount, truncate_reward_amount(expected_del1_reward));

            // new reward after withdrawal
            let rewards2 = simulator.simulate_epoch_single_node(node_params).unwrap();
            let expected_operator2 = "1411250.1907492676".parse().unwrap();
            let expected_delegator_reward2 = "2200004.051009689".parse().unwrap();
            compare_decimals(rewards2.delegates, expected_delegator_reward2, None);
            compare_decimals(rewards2.operator, expected_operator2, None);
            check_rewarding_invariant(&simulator);

            // check final values
            let node = simulator.nodes.get_mut(&0).unwrap();
            let delegation1 = node.delegations.get_mut("alice").unwrap();

            let reward_del1 = node
                .rewarding_details
                .withdraw_delegator_reward(delegation1)
                .unwrap();
            let expected_del1_reward = "1799970.5883041779".parse().unwrap();
            assert_eq!(
                reward_del1.amount,
                truncate_reward_amount(expected_del1_reward)
            );

            let delegation2 = node.delegations.get_mut("bob").unwrap();
            let reward_del2 = node
                .rewarding_details
                .withdraw_delegator_reward(delegation2)
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

            let mut work_factor = simulator.legacy_active_work_factor();
            let mut performance = Percent::from_percentage_value(100).unwrap();
            for epoch in 0..720 {
                if epoch == 0 {
                    simulator
                        .delegate("a", Coin::new(18000_000000, "unym"), 0)
                        .unwrap()
                }
                if epoch == 42 {
                    simulator
                        .delegate("b", Coin::new(2000_000000, "unym"), 0)
                        .unwrap()
                }
                if epoch == 89 {
                    work_factor = simulator.legacy_standby_work_factor();
                }
                if epoch == 123 {
                    simulator
                        .delegate("c", Coin::new(6666_000000, "unym"), 0)
                        .unwrap()
                }
                if epoch == 167 {
                    performance = Percent::from_percentage_value(90).unwrap();
                }
                if epoch == 245 {
                    simulator
                        .delegate("d", Coin::new(2050_000000, "unym"), 0)
                        .unwrap()
                }
                if epoch == 264 {
                    let (delegation, _reward) = simulator.undelegate("b", 0).unwrap();
                    // sanity check to make sure we undelegated what we wanted to undelegate : )
                    assert_eq!(delegation.amount.u128(), 2000_000000);
                    // TODO: figure out if there's a good way to verify whether `reward` is what we expect it to be
                }
                if epoch == 345 {
                    work_factor = simulator.legacy_active_work_factor();
                }
                if epoch == 358 {
                    performance = Percent::from_percentage_value(100).unwrap();
                }
                if epoch == 458 {
                    let (delegation, _reward) = simulator.undelegate("a", 0).unwrap();
                    // sanity check to make sure we undelegated what we wanted to undelegate : )
                    assert_eq!(delegation.amount.u128(), 18000_000000);
                    // TODO: figure out if there's a good way to verify whether `reward` is what we expect it to be
                }
                if epoch == 545 {
                    simulator
                        .delegate("e", Coin::new(5000_000000, "unym"), 0)
                        .unwrap()
                }

                // this has to always hold
                check_rewarding_invariant(&simulator);
                let node_params = NodeRewardingParameters::new(performance, work_factor);
                simulator.simulate_epoch_single_node(node_params).unwrap();
            }

            // after everyone undelegates, there should be nothing left in the delegates pool
            simulator.undelegate("c", 0).unwrap();
            simulator.undelegate("d", 0).unwrap();
            simulator.undelegate("e", 0).unwrap();

            let node = &simulator.nodes[&0];
            assert_eq!(Decimal::zero(), node.rewarding_details.delegates);
        }
    }

    #[test]
    #[allow(clippy::inconsistent_digit_grouping)]
    fn multiple_nodes_against_known_values() {
        // TODO: this test can be further improved by checking values after EVERY interval
        // rather than just checking the final results

        let epochs_in_interval = 1u32;
        let interval_pool_emission = Percent::from_percentage_value(2).unwrap();

        // the import values here are active set being 6 and rewarded set being 10
        // since those are the values we were using in the past
        let rewarded_set = RewardedSetParams {
            entry_gateways: 1,
            exit_gateways: 2,
            mixnodes: 3,
            standby: 4,
        };

        let reward_pool = 250_000_000_000_000u128;
        let staking_supply = 100_000_000_000_000u128;
        let epoch_reward_budget =
            interval_pool_emission * Decimal::from_ratio(reward_pool, epochs_in_interval);
        let stake_saturation_point =
            Decimal::from_ratio(staking_supply, rewarded_set.rewarded_set_size());

        let rewarding_params = RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: Decimal::from_atomics(reward_pool, 0).unwrap(),
                staking_supply: Decimal::from_atomics(staking_supply, 0).unwrap(),
                staking_supply_scale_factor: Percent::hundred(),
                epoch_reward_budget,
                stake_saturation_point,
                sybil_resistance: Percent::from_percentage_value(30).unwrap(),
                active_set_work_factor: Decimal::percent(1000), // value '10'
                interval_pool_emission,
            },
            rewarded_set,
        };

        let interval = Interval::init_interval(
            epochs_in_interval,
            Duration::from_secs(60 * 60),
            &mock_env(),
        );

        let mut simulator = Simulator::new(rewarding_params, interval);

        let n0 = simulator
            .bond(
                Coin::new(11_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(1_000_000_000000, "unym"), n0)
            .unwrap();

        let n1 = simulator
            .bond(
                Coin::new(1_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(11_000_000_000000, "unym"), n1)
            .unwrap();

        let n2 = simulator
            .bond(
                Coin::new(1_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(9_000_000_000000, "unym"), n2)
            .unwrap();

        let n3 = simulator
            .bond(
                Coin::new(1_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(0).unwrap(),
                    interval_operating_cost: Coin::new(500_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(7_000_000_000000, "unym"), n3)
            .unwrap();

        let n4 = simulator
            .bond(
                Coin::new(1000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(7_999_000_000000, "unym"), n4)
            .unwrap();

        let n5 = simulator
            .bond(
                Coin::new(1_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(7_000_000_000000, "unym"), n5)
            .unwrap();

        let n6 = simulator
            .bond(
                Coin::new(11_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(1_000_000_000000, "unym"), n6)
            .unwrap();

        let n7 = simulator
            .bond(
                Coin::new(1_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(9_000_000_000000, "unym"), n7)
            .unwrap();

        let n8 = simulator
            .bond(
                Coin::new(1_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(0).unwrap(),
                    interval_operating_cost: Coin::new(500_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(7_000_000_000000, "unym"), n8)
            .unwrap();

        let n9 = simulator
            .bond(
                Coin::new(1_000_000_000000, "unym"),
                NodeCostParams {
                    profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                    interval_operating_cost: Coin::new(40_000_000, "unym"),
                },
            )
            .unwrap();
        simulator
            .delegate("delegator", Coin::new(7_000_000_000000, "unym"), n9)
            .unwrap();

        let uptime_1 = Percent::from_percentage_value(100).unwrap();
        let uptime_09 = Percent::from_percentage_value(90).unwrap();
        let uptime_0 = Percent::from_percentage_value(0).unwrap();

        let active_work = simulator.legacy_active_work_factor();
        let standby_work = simulator.legacy_standby_work_factor();

        let node_params = [
            (n0, NodeRewardingParameters::new(uptime_1, active_work)),
            (n1, NodeRewardingParameters::new(uptime_1, active_work)),
            (n2, NodeRewardingParameters::new(uptime_1, active_work)),
            (n3, NodeRewardingParameters::new(uptime_09, active_work)),
            (n4, NodeRewardingParameters::new(uptime_09, active_work)),
            (n5, NodeRewardingParameters::new(uptime_0, active_work)),
            (n6, NodeRewardingParameters::new(uptime_1, standby_work)),
            (n7, NodeRewardingParameters::new(uptime_1, standby_work)),
            (n8, NodeRewardingParameters::new(uptime_09, standby_work)),
            (n9, NodeRewardingParameters::new(uptime_0, standby_work)),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();

        for _ in 0..23 {
            simulator.simulate_full_interval(&node_params).unwrap();
        }

        // we allow the delta to be within 0.1unym,
        // because the expected values, especially for the reward pool,
        // do not provide us with any higher precision
        let epsilon = Some(Decimal::from_ratio(1u32, 10u32));

        let expected_reward_pool = "184876811322111.7".parse().unwrap();
        let expected_staking_supply = "165123188677888.3".parse().unwrap();
        compare_decimals(
            expected_reward_pool,
            simulator.system_rewarding_params.interval.reward_pool,
            epsilon,
        );
        compare_decimals(
            expected_staking_supply,
            simulator.system_rewarding_params.interval.staking_supply,
            epsilon,
        );

        let expected_n0_pledge = "24307061704726.808".parse().unwrap();
        let expected_n0_delegated = "2031528592775.6752".parse().unwrap();
        let node = &simulator.nodes[&0].rewarding_details;
        compare_decimals(node.operator, expected_n0_pledge, epsilon);
        compare_decimals(node.delegates, expected_n0_delegated, epsilon);

        let expected_n1_pledge = "3544171010629.92".parse().unwrap();
        let expected_n1_delegated = "20854154351479.96".parse().unwrap();
        let node = &simulator.nodes[&1].rewarding_details;
        compare_decimals(node.operator, expected_n1_pledge, epsilon);
        compare_decimals(node.delegates, expected_n1_delegated, epsilon);

        let expected_n2_pledge = "3781120900089.8865".parse().unwrap();
        let expected_n2_delegated = "18634530734287.53".parse().unwrap();
        let node = &simulator.nodes[&2].rewarding_details;
        compare_decimals(node.operator, expected_n2_pledge, epsilon);
        compare_decimals(node.delegates, expected_n2_delegated, epsilon);

        let expected_n3_pledge = "2313562111772.3165".parse().unwrap();
        let expected_n3_delegated = "16090515100131.858".parse().unwrap();
        let node = &simulator.nodes[&3].rewarding_details;
        compare_decimals(node.operator, expected_n3_pledge, epsilon);
        compare_decimals(node.delegates, expected_n3_delegated, epsilon);

        let expected_n4_pledge = "1419679306492.7962".parse().unwrap();
        let expected_n4_delegated = "16802494863659.93".parse().unwrap();
        let node = &simulator.nodes[&4].rewarding_details;
        compare_decimals(node.operator, expected_n4_pledge, epsilon);
        compare_decimals(node.delegates, expected_n4_delegated, epsilon);

        let expected_n5_pledge = "1000000000000".parse().unwrap();
        let expected_n5_delegated = "7000000000000".parse().unwrap();
        let node = &simulator.nodes[&5].rewarding_details;
        compare_decimals(node.operator, expected_n5_pledge, epsilon);
        compare_decimals(node.delegates, expected_n5_delegated, epsilon);

        let expected_n6_pledge = "14114996375922.574".parse().unwrap();
        let expected_n6_delegated = "1249173915284.053".parse().unwrap();
        let node = &simulator.nodes[&6].rewarding_details;
        compare_decimals(node.operator, expected_n6_pledge, epsilon);
        compare_decimals(node.delegates, expected_n6_delegated, epsilon);

        let expected_n7_pledge = "1225564192694.3525".parse().unwrap();
        let expected_n7_delegated = "9931461332688.53".parse().unwrap();
        let node = &simulator.nodes[&7].rewarding_details;
        compare_decimals(node.operator, expected_n7_pledge, epsilon);
        compare_decimals(node.delegates, expected_n7_delegated, epsilon);

        let expected_n8_pledge = "1112319106593.8608".parse().unwrap();
        let expected_n8_delegated = "7710855078658.264".parse().unwrap();
        let node = &simulator.nodes[&8].rewarding_details;
        compare_decimals(node.operator, expected_n8_pledge, epsilon);
        compare_decimals(node.delegates, expected_n8_delegated, epsilon);

        let expected_n9_pledge = "1000000000000".parse().unwrap();
        let expected_n9_delegated = "7000000000000".parse().unwrap();
        let node = &simulator.nodes[&9].rewarding_details;
        compare_decimals(node.operator, expected_n9_pledge, epsilon);
        compare_decimals(node.delegates, expected_n9_delegated, epsilon);
    }
}
