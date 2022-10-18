// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::{MixNodeCostParams, MixNodeRewarding};
use crate::reward_params::{IntervalRewardParams, NodeRewardParams, RewardingParams};
use crate::{Delegation, EpochId, Interval, MixId};
use cosmwasm_std::{Addr, Coin, Decimal};
use std::collections::BTreeMap;

pub struct SimulatedNode {
    pub rewarding_details: MixNodeRewarding,
    pub delegations: Vec<Delegation>,
}

impl SimulatedNode {
    pub fn new(
        cost_params: MixNodeCostParams,
        initial_pledge: &Coin,
        current_epoch: EpochId,
    ) -> Self {
        SimulatedNode {
            rewarding_details: MixNodeRewarding::initialise_new(
                cost_params,
                initial_pledge,
                current_epoch,
            ),
            delegations: vec![],
        }
    }
}

pub struct MultiNodeSimulator {
    pub nodes: BTreeMap<MixId, SimulatedNode>,
    pub system_rewarding_params: RewardingParams,
    pub interval: Interval,

    next_mix_id: MixId,
    pending_reward_pool_emission: Decimal,
}

impl MultiNodeSimulator {
    pub fn new(system_rewarding_params: RewardingParams, interval: Interval) -> Self {
        MultiNodeSimulator {
            nodes: Default::default(),
            system_rewarding_params,
            interval,
            next_mix_id: 0,
            pending_reward_pool_emission: Default::default(),
        }
    }

    fn advance_epoch(&mut self) {
        let updated = self.interval.advance_epoch();

        // we rolled over an interval
        if self.interval.current_interval_id() + 1 == updated.current_interval_id() {
            let old = self.system_rewarding_params.interval;
            let reward_pool = old.reward_pool - self.pending_reward_pool_emission;
            let staking_supply = old.staking_supply + self.pending_reward_pool_emission;
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
            self.pending_reward_pool_emission = Decimal::zero();
        }
        self.interval = updated;
    }

    pub fn bond(&mut self, pledge: Coin, cost_params: MixNodeCostParams) -> MixId {
        self.next_mix_id += 1;
        let mix_id = self.next_mix_id;

        self.nodes.insert(
            mix_id,
            SimulatedNode::new(
                cost_params,
                &pledge,
                self.interval.current_epoch_absolute_id(),
            ),
        );

        mix_id
    }

    pub fn delegate<S: Into<String>>(&mut self, delegator: S, delegation: Coin, mix_id: MixId) {
        let node = self.nodes.get_mut(&mix_id).expect("node doesn't exist");
        node.rewarding_details
            .add_base_delegation(delegation.amount);

        let delegation = Delegation::new(
            Addr::unchecked(delegator),
            mix_id,
            node.rewarding_details.total_unit_reward,
            delegation,
            42,
            None,
        );

        node.delegations.push(delegation)
    }

    pub fn simulate_epoch(&mut self, node_params: &BTreeMap<MixId, NodeRewardParams>) {
        let mut params_keys = node_params.keys().copied().collect::<Vec<_>>();
        params_keys.sort_unstable();
        let mut node_keys = self.nodes.keys().copied().collect::<Vec<_>>();
        node_keys.sort_unstable();

        if params_keys != node_keys {
            panic!("invalid node rewarding params provided");
        }

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
        }

        self.advance_epoch();
    }

    // assume node state doesn't change in the interval (kinda unrealistic)
    pub fn simulate_interval(&mut self, node_params: &BTreeMap<MixId, NodeRewardParams>) {
        for _ in 0..self.interval.epochs_in_interval() {
            self.simulate_epoch(node_params)
        }
    }

    pub fn print_state(&self) {
        println!(
            "reward pool:\t\t\t{}",
            self.system_rewarding_params.interval.reward_pool
        );
        println!(
            "staking supply pool:\t{}",
            self.system_rewarding_params.interval.staking_supply
        );
        println!();

        for (mix_id, node) in &self.nodes {
            println!("Node {}", mix_id);
            println!("Total bond:\t\t\t{}", node.rewarding_details.operator);
            println!("Total delegations:\t{}", node.rewarding_details.delegates);
        }
    }
}

#[cfg(test)]
mod tests {

    #![allow(clippy::inconsistent_digit_grouping)]

    use super::*;
    use crate::Percent;
    use cosmwasm_std::testing::mock_env;
    use std::time::Duration;

    #[test]
    fn run_multi_node_simulator_against_known_values() {
        let epochs_in_interval = 1u32;
        let rewarded_set_size = 10;
        let active_set_size = 6;
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

        let mut simulator = MultiNodeSimulator::new(rewarding_params, interval);

        let n0 = simulator.bond(
            Coin::new(11_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(1_000_000_000000, "unym"), n0);

        let n1 = simulator.bond(
            Coin::new(1_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(11_000_000_000000, "unym"), n1);

        let n2 = simulator.bond(
            Coin::new(1_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(9_000_000_000000, "unym"), n2);

        let n3 = simulator.bond(
            Coin::new(1_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(0).unwrap(),
                interval_operating_cost: Coin::new(500_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(7_000_000_000000, "unym"), n3);

        let n4 = simulator.bond(
            Coin::new(1000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(7_999_000_000000, "unym"), n4);

        let n5 = simulator.bond(
            Coin::new(1_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(7_000_000_000000, "unym"), n5);

        let n6 = simulator.bond(
            Coin::new(11_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(1_000_000_000000, "unym"), n6);

        let n7 = simulator.bond(
            Coin::new(1_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(9_000_000_000000, "unym"), n7);

        let n8 = simulator.bond(
            Coin::new(1_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(0).unwrap(),
                interval_operating_cost: Coin::new(500_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(7_000_000_000000, "unym"), n8);

        let n9 = simulator.bond(
            Coin::new(1_000_000_000000, "unym"),
            MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
                interval_operating_cost: Coin::new(40_000_000, "unym"),
            },
        );
        simulator.delegate("delegator", Coin::new(7_000_000_000000, "unym"), n9);

        let uptime_1 = Percent::from_percentage_value(100).unwrap();
        let uptime_09 = Percent::from_percentage_value(90).unwrap();
        let uptime_0 = Percent::from_percentage_value(0).unwrap();

        let node_params = [
            (n0, NodeRewardParams::new(uptime_1, true)),
            (n1, NodeRewardParams::new(uptime_1, true)),
            (n2, NodeRewardParams::new(uptime_1, true)),
            (n3, NodeRewardParams::new(uptime_09, true)),
            (n4, NodeRewardParams::new(uptime_09, true)),
            (n5, NodeRewardParams::new(uptime_0, true)),
            (n6, NodeRewardParams::new(uptime_1, false)),
            (n7, NodeRewardParams::new(uptime_1, false)),
            (n8, NodeRewardParams::new(uptime_09, false)),
            (n9, NodeRewardParams::new(uptime_0, false)),
        ]
        .into_iter()
        .collect::<BTreeMap<_, _>>();

        for _ in 0..24 {
            simulator.simulate_interval(&node_params);
        }

        simulator.print_state();
    }
}
