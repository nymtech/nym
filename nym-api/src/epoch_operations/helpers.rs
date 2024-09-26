// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use cosmwasm_std::{Decimal, Fraction};
use nym_mixnet_contract_common::reward_params::{NodeRewardingParameters, Performance, WorkFactor};
use nym_mixnet_contract_common::{ExecuteMsg, Interval, NodeId, RewardedSet, RewardingParams};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct NodeWithPerformance {
    pub(crate) node_id: NodeId,
    pub(crate) performance: Performance,
}

impl NodeWithPerformance {
    pub fn with_work(self, work_factor: WorkFactor) -> RewardedNodeWithParams {
        RewardedNodeWithParams {
            node_id: self.node_id,
            params: NodeRewardingParameters {
                performance: self.performance,
                work_factor,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RewardedNodeWithParams {
    pub(crate) node_id: NodeId,
    pub(crate) params: NodeRewardingParameters,
}

impl From<RewardedNodeWithParams> for ExecuteMsg {
    fn from(node_reward: RewardedNodeWithParams) -> Self {
        ExecuteMsg::RewardNode {
            node_id: node_reward.node_id,
            params: node_reward.params,
        }
    }
}

pub(super) fn stake_to_f64(stake: Decimal) -> f64 {
    let max = f64::MAX.round() as u128;

    let num = stake.numerator().u128();
    let den = stake.denominator().u128();

    if num > max || den > max {
        // we know actual stake can't possibly exceed 1B, so worst case scenario just use integer rounding
        (num / den) as f64
    } else {
        (num as f64) / (den as f64)
    }
}

impl EpochAdvancer {
    pub(crate) async fn load_mixnode_performance(
        &self,
        interval: &Interval,
        node_id: NodeId,
    ) -> NodeWithPerformance {
        let uptime = self
            .storage
            .get_average_mixnode_uptime_in_the_last_24hrs(
                node_id,
                interval.current_epoch_end_unix_timestamp(),
            )
            .await
            .unwrap_or_default();

        NodeWithPerformance {
            node_id,
            performance: uptime.into(),
        }
    }

    pub(crate) async fn load_gateway_performance(
        &self,
        interval: &Interval,
        node_id: NodeId,
    ) -> NodeWithPerformance {
        let uptime = self
            .storage
            .get_average_gateway_uptime_in_the_last_24hrs(
                node_id,
                interval.current_epoch_end_unix_timestamp(),
            )
            .await
            .unwrap_or_default();

        NodeWithPerformance {
            node_id,
            performance: uptime.into(),
        }
    }

    pub(crate) async fn load_any_performance(
        &self,
        interval: &Interval,
        node_id: NodeId,
    ) -> NodeWithPerformance {
        // currently we can't do much better without new network monitor
        let mix_performance = self.load_mixnode_performance(interval, node_id).await;
        if !mix_performance.performance.is_zero() {
            return mix_performance;
        }

        self.load_gateway_performance(interval, node_id).await
    }

    pub(crate) async fn load_nodes_for_rewarding(
        &self,
        interval: &Interval,
        nodes: &RewardedSet,
        // we only need reward parameters for active set work factor and rewarded/active set sizes;
        // we do not need exact values of reward pool, staking supply, etc., so it's fine if it's slightly out of sync
        global_rewarding_params: RewardingParams,
    ) -> Vec<RewardedNodeWithParams> {
        // currently we are using constant omega for nodes, but that will change with tickets
        // or different reward split between entry, exit, etc. at that point this will have to be calculated elsewhere
        let active_node_work_factor = global_rewarding_params.active_node_work();
        let standby_node_work_factor = global_rewarding_params.standby_node_work();

        // SANITY CHECK:
        let standby_share = Decimal::from_atomics(nodes.standby.len() as u128, 0).unwrap()
            * standby_node_work_factor;
        let active_share = Decimal::from_atomics(nodes.active_set_size() as u128, 0).unwrap()
            * active_node_work_factor;
        let total_work = standby_share + active_share;

        // this HAS TO blow up. there's no recovery
        assert!(total_work <= Decimal::one(), "work calculation logic is flawed! somehow the total work in the system is greater than 1!");

        let mut with_performance = Vec::with_capacity(nodes.rewarded_set_size());

        // all the active set mixnodes
        for node_id in nodes
            .layer1
            .iter()
            .chain(nodes.layer2.iter())
            .chain(nodes.layer3.iter())
        {
            with_performance.push(
                self.load_mixnode_performance(interval, *node_id)
                    .await
                    .with_work(active_node_work_factor),
            )
        }

        // all the active set gateways
        for node_id in nodes
            .entry_gateways
            .iter()
            .chain(nodes.exit_gateways.iter())
        {
            with_performance.push(
                self.load_gateway_performance(interval, *node_id)
                    .await
                    .with_work(active_node_work_factor),
            )
        }

        // all the standby nodes
        for node_id in &nodes.standby {
            with_performance.push(
                self.load_any_performance(interval, *node_id)
                    .await
                    .with_work(standby_node_work_factor),
            )
        }

        with_performance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare_large_floats(a: f64, b: f64) {
        // for very large floats, allow for smaller larger epsilon
        let epsilon = if a > 100_000_000_000f64 {
            0.1
        } else {
            0.0000000001
        };

        if a > b {
            assert!(a - b < epsilon, "{} != {}", a, b)
        } else {
            assert!(b - a < epsilon, "{} != {}", a, b)
        }
    }

    #[test]
    fn decimal_stake_to_f64() {
        let raw = vec![
            ("0.1", 0.1f64),
            ("0.01", 0.01f64),
            ("0.001", 0.001f64),
            ("0.0001", 0.0001f64),
            ("0.00001", 0.00001f64),
            ("1.000001", 1.000001f64),
            ("10.000001", 10.000001f64),
            ("100.000001", 100.000001f64),
            ("1000.000001", 1000.000001f64),
            ("10000.000001", 10000.000001f64),
            ("100000.000001", 100000.000001f64),
            ("1000000.000001", 1000000.000001f64),
            ("10000000.000001", 10000000.000001f64),
            ("100000000.000001", 100000000.000001f64),
            ("1000000000.000001", 1000000000.000001f64),
            ("10000000000.000001", 10000000000.000001f64),
            ("100000000000.12345", 100000000000.12345f64),
            ("1000000000000.000001", 1000000000000.000001f64),
            ("123456789123456.789123456", 123_456_789_123_456.8_f64),
        ];

        for (raw_decimal, expected_f64) in raw {
            let decimal: Decimal = raw_decimal.parse().unwrap();
            compare_large_floats(expected_f64, stake_to_f64(decimal))
        }
    }
}
