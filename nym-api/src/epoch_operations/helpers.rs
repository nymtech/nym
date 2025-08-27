// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::EpochAdvancer;
use crate::support::caching::Cache;
use cosmwasm_std::{Decimal, Fraction};
use nym_api_requests::models::NodeAnnotation;
use nym_mixnet_contract_common::helpers::IntoBaseDecimal;
use nym_mixnet_contract_common::reward_params::{NodeRewardingParameters, Performance, WorkFactor};
use nym_mixnet_contract_common::{
    EpochRewardedSet, ExecuteMsg, NodeId, RewardedSet, RewardingParams,
};
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::HashMap;
use tokio::sync::RwLockReadGuard;
use tracing::{debug, error};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct NodeWithPerformance {
    pub(crate) node_id: NodeId,
    pub(crate) performance: Performance,
}

impl NodeWithPerformance {
    pub fn new(node_id: NodeId, performance: Performance) -> Self {
        NodeWithPerformance {
            node_id,
            performance,
        }
    }

    pub fn new_zero(node_id: NodeId) -> Self {
        NodeWithPerformance {
            node_id,
            performance: Default::default(),
        }
    }

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

struct PerNodeWork {
    active: WorkFactor,
    standby: WorkFactor,
}

struct NodeWorkCalculationComponents {
    active_set_size: Decimal,
    standby_set_size: Decimal,
    per_node_work: PerNodeWork,
}

impl NodeWorkCalculationComponents {
    fn standby_set_work_share(&self) -> Decimal {
        self.standby_set_size * self.per_node_work.standby
    }

    fn active_set_work_share(&self) -> Decimal {
        self.active_set_size * self.per_node_work.active
    }
}

fn default_node_work_calculation(
    nodes: &RewardedSet,
    global_rewarding_params: RewardingParams,
) -> NodeWorkCalculationComponents {
    let per_node_work = PerNodeWork {
        active: global_rewarding_params.active_node_work(),
        standby: global_rewarding_params.standby_node_work(),
    };
    // SANITY CHECK:
    // SAFETY: 0 decimal places is within the range of `Decimal`
    #[allow(clippy::unwrap_used)]
    let standby_set_size = Decimal::from_atomics(nodes.standby.len() as u128, 0).unwrap();
    #[allow(clippy::unwrap_used)]
    let active_set_size = Decimal::from_atomics(nodes.active_set_size() as u128, 0).unwrap();

    NodeWorkCalculationComponents {
        active_set_size,
        standby_set_size,
        per_node_work,
    }
}

fn manual_node_work_calculation(
    nodes: &RewardedSet,
    global_rewarding_params: RewardingParams,
) -> NodeWorkCalculationComponents {
    // calculate everything manually based on the actual rewarded set on hand
    // but always attempt to minimise the node work, so take the maximum values
    // of the set sizes between new and old parameters
    // (more nodes = smaller per-node work as it has to be spread through more entries)
    let rewarded_set_size = max(
        global_rewarding_params.rewarded_set.rewarded_set_size(),
        nodes.rewarded_set_size() as u32,
    );
    let standby_set_size = max(
        global_rewarding_params.rewarded_set.standby,
        nodes.standby_set_size() as u32,
    );
    // the unwraps here are fine as we're guaranteed an `u32` is going to fit in a Decimal with 0 decimal places
    #[allow(clippy::unwrap_used)]
    let rewarded_set_size_dec = rewarded_set_size.into_base_decimal().unwrap();
    #[allow(clippy::unwrap_used)]
    let standby_set_size_dec = standby_set_size.into_base_decimal().unwrap();
    #[allow(clippy::unwrap_used)]
    let active_set_size = rewarded_set_size
        .saturating_sub(standby_set_size)
        .into_base_decimal()
        .unwrap();

    let standby_node_work = global_rewarding_params
        .interval
        .standby_node_work(rewarded_set_size_dec, standby_set_size_dec);
    let active_node_work = global_rewarding_params
        .interval
        .active_node_work(standby_node_work);
    let per_node_work = PerNodeWork {
        active: active_node_work,
        standby: standby_node_work,
    };

    NodeWorkCalculationComponents {
        active_set_size,
        standby_set_size: standby_set_size_dec,
        per_node_work,
    }
}

fn determine_per_node_work(
    nodes: &RewardedSet,
    // we only need reward parameters for active set work factor and rewarded/active set sizes;
    // we do not need exact values of reward pool, staking supply, etc., so it's fine if it's slightly out of sync
    global_rewarding_params: RewardingParams,
) -> PerNodeWork {
    // currently we are using constant omega for nodes, but that will change with tickets
    // or different reward split between entry, exit, etc. at that point this will have to be calculated elsewhere
    let res = if nodes.matches_parameters(global_rewarding_params.rewarded_set) {
        default_node_work_calculation(nodes, global_rewarding_params)
    } else {
        error!("the current rewarded set does not much current rewarding parameters. this could only be expected if rewarded set distribution has been changed mid-epoch");
        manual_node_work_calculation(nodes, global_rewarding_params)
    };

    let active_node_work_factor = res.per_node_work.active;
    let standby_node_work_factor = res.per_node_work.standby;

    debug!("using {active_node_work_factor} as active node work factor and {standby_node_work_factor} as standby node work factor");

    let standby_share = res.standby_set_work_share();
    let active_share = res.active_set_work_share();
    let total_work = standby_share + active_share;

    // this HAS TO blow up. there's no recovery
    #[allow(clippy::panic)]
    if total_work > Decimal::one() {
        panic!("work calculation logic is flawed! somehow the total work in the system is greater than 1! \
            total work={total_work}, \
            active set share={active_share}, \
            standby share={standby_share}, \
            active node work factor={active_node_work_factor}, \
            standby node work factor={standby_node_work_factor}, \
            active set size={} \
            standby set size={}", res.active_set_size, res.standby_set_size);
    }

    PerNodeWork {
        active: active_node_work_factor,
        standby: standby_node_work_factor,
    }
}

impl EpochAdvancer {
    fn load_performance(
        status_cache: &Option<RwLockReadGuard<Cache<HashMap<NodeId, NodeAnnotation>>>>,
        node_id: NodeId,
    ) -> NodeWithPerformance {
        let Some(status_cache) = status_cache.as_ref() else {
            return NodeWithPerformance::new_zero(node_id);
        };

        match status_cache.get(&node_id) {
            Some(annotation) => NodeWithPerformance::new(
                node_id,
                annotation.detailed_performance.to_rewarding_performance(),
            ),
            None => NodeWithPerformance::new_zero(node_id),
        }
    }

    pub(crate) async fn load_nodes_for_rewarding(
        &self,
        nodes: &EpochRewardedSet,
        // we only need reward parameters for active set work factor and rewarded/active set sizes;
        // we do not need exact values of reward pool, staking supply, etc., so it's fine if it's slightly out of sync
        global_rewarding_params: RewardingParams,
    ) -> Vec<RewardedNodeWithParams> {
        let nodes = &nodes.assignment;
        let nodes_work = determine_per_node_work(nodes, global_rewarding_params);
        let active_node_work_factor = nodes_work.active;
        let standby_node_work_factor = nodes_work.standby;

        let status_cache = self.status_cache.node_annotations().await;
        if status_cache.is_none() {
            error!("there are no node annotations available");
        };

        let mut with_performance = Vec::with_capacity(nodes.rewarded_set_size());

        // all the active set mixnodes
        for &node_id in nodes
            .layer1
            .iter()
            .chain(nodes.layer2.iter())
            .chain(nodes.layer3.iter())
        {
            with_performance.push(
                Self::load_performance(&status_cache, node_id).with_work(active_node_work_factor),
            );
        }

        // all the active set gateways
        for &node_id in nodes
            .entry_gateways
            .iter()
            .chain(nodes.exit_gateways.iter())
        {
            with_performance.push(
                Self::load_performance(&status_cache, node_id).with_work(active_node_work_factor),
            );
        }

        // all the standby nodes
        for &node_id in &nodes.standby {
            with_performance.push(
                Self::load_performance(&status_cache, node_id).with_work(standby_node_work_factor),
            );
        }

        with_performance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_contracts_common::Percent;
    use nym_mixnet_contract_common::reward_params::RewardedSetParams;
    use nym_mixnet_contract_common::IntervalRewardParams;

    fn compare_large_floats(a: f64, b: f64) {
        // for very large floats, allow for smaller larger epsilon
        let epsilon = if a > 100_000_000_000f64 {
            0.1
        } else {
            0.0000000001
        };

        if a > b {
            assert!(a - b < epsilon, "{a} != {b}")
        } else {
            assert!(b - a < epsilon, "{a} != {b}")
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

    fn dummy_rewarding_params() -> RewardingParams {
        RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: Decimal::from_atomics(100_000_000_000_000u128, 0).unwrap(),
                staking_supply: Decimal::from_atomics(123_456_000_000_000u128, 0).unwrap(),
                staking_supply_scale_factor: Percent::hundred(),
                epoch_reward_budget: Decimal::from_ratio(100_000_000_000_000u128, 1234u32)
                    * Decimal::percent(1),
                stake_saturation_point: Decimal::from_ratio(123_456_000_000_000u128, 313u32),
                sybil_resistance: Percent::from_percentage_value(23).unwrap(),
                active_set_work_factor: Decimal::from_atomics(10u32, 0).unwrap(),
                interval_pool_emission: Percent::from_percentage_value(1).unwrap(),
            },
            rewarded_set: RewardedSetParams {
                entry_gateways: 50,
                exit_gateways: 70,
                mixnodes: 120,
                standby: 20,
            },
        }
    }

    #[test]
    fn determining_nodes_work() {
        let params = dummy_rewarding_params();
        // matched parameters
        let rewarded_set = RewardedSet {
            entry_gateways: (1..)
                .take(params.rewarded_set.entry_gateways as usize)
                .collect(),
            exit_gateways: (1000..)
                .take(params.rewarded_set.exit_gateways as usize)
                .collect(),
            layer1: (2000..)
                .take(params.rewarded_set.mixnodes as usize / 3)
                .collect(),
            layer2: (3000..)
                .take(params.rewarded_set.mixnodes as usize / 3)
                .collect(),
            layer3: (4000..)
                .take(params.rewarded_set.mixnodes as usize / 3)
                .collect(),
            standby: (5000..)
                .take(params.rewarded_set.standby as usize)
                .collect(),
        };

        let work = determine_per_node_work(&rewarded_set, params);
        assert_eq!(work.active, params.active_node_work());
        assert_eq!(work.standby, params.standby_node_work());

        // updated
        // here we're interested in the fact that the calculation does not panic, i.e. total work <= 1
        let params = dummy_rewarding_params();
        let rewarded_set = RewardedSet {
            entry_gateways: (1..).take(250).collect(),
            exit_gateways: (1000..).take(100).collect(),
            layer1: (2000..).take(10).collect(),
            layer2: (3000..).take(10).collect(),
            layer3: (4000..).take(10).collect(),
            standby: (5000..).take(5).collect(),
        };

        let work = determine_per_node_work(&rewarded_set, params);
        assert_ne!(work.active, params.active_node_work());
        assert_ne!(work.standby, params.standby_node_work());
    }
}
