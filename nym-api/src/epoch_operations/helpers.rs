// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::epoch_operations::RewardedSetUpdater;
use cosmwasm_std::{Decimal, Fraction};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{ExecuteMsg, Interval, MixId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct MixnodeWithPerformance {
    pub(crate) mix_id: MixId,

    pub(crate) performance: Performance,
}

impl From<MixnodeWithPerformance> for ExecuteMsg {
    fn from(mix_reward: MixnodeWithPerformance) -> Self {
        ExecuteMsg::RewardMixnode {
            mix_id: mix_reward.mix_id,
            performance: mix_reward.performance,
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

impl RewardedSetUpdater {
    pub(crate) async fn load_performance(
        &self,
        interval: &Interval,
        mix_id: MixId,
    ) -> MixnodeWithPerformance {
        let uptime = self
            .storage
            .get_average_mixnode_uptime_in_the_last_24hrs(
                mix_id,
                interval.current_epoch_end_unix_timestamp(),
            )
            .await
            .unwrap_or_default();

        MixnodeWithPerformance {
            mix_id,
            performance: uptime.into(),
        }
    }

    pub(crate) async fn load_nodes_performance(
        &self,
        interval: &Interval,
        nodes: &[MixId],
    ) -> Vec<MixnodeWithPerformance> {
        let mut with_performance = Vec::with_capacity(nodes.len());
        for mix_id in nodes {
            with_performance.push(self.load_performance(interval, *mix_id).await)
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
