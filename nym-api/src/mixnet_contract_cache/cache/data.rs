// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::{ConfigScoreDataResponse, LegacyGatewayBondWithId};
use nym_mixnet_contract_common::{
    ConfigScoreParams, GatewayBond, HistoricalNymNodeVersionEntry, Interval, KeyRotationState,
    MixNodeDetails, NodeId, NymNodeDetails, RewardingParams,
};
use nym_topology::CachedEpochRewardedSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ConfigScoreData {
    pub(crate) config_score_params: ConfigScoreParams,
    pub(crate) nym_node_version_history: Vec<HistoricalNymNodeVersionEntry>,
}

impl From<ConfigScoreData> for ConfigScoreDataResponse {
    fn from(value: ConfigScoreData) -> Self {
        ConfigScoreDataResponse {
            parameters: value.config_score_params.into(),
            version_history: value
                .nym_node_version_history
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

/// A legacy gateway bond together with its node id, as held in the cache.
///
/// Deliberately not the [`LegacyGatewayBondWithId`] response type: that one `#[serde(flatten)]`s
/// its bond to keep the JSON shape flat, and a flattened field makes serde ask the serialiser for
/// an unknown-length map, which bincode (this cache's on-disk format) cannot encode. Keeping the
/// stored shape separate from the wire shape lets each pick what its format needs.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct CachedLegacyGatewayBond {
    pub(crate) bond: GatewayBond,
    pub(crate) node_id: NodeId,
}

impl From<CachedLegacyGatewayBond> for LegacyGatewayBondWithId {
    fn from(cached: CachedLegacyGatewayBond) -> Self {
        LegacyGatewayBondWithId {
            bond: cached.bond,
            node_id: cached.node_id,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct MixnetContractCacheData {
    pub(crate) rewarding_denom: String,

    pub(crate) legacy_mixnodes: Vec<MixNodeDetails>,
    pub(crate) legacy_gateways: Vec<CachedLegacyGatewayBond>,
    pub(crate) nym_nodes: Vec<NymNodeDetails>,
    pub(crate) rewarded_set: CachedEpochRewardedSet,

    pub(crate) config_score_data: ConfigScoreData,
    pub(crate) current_reward_params: RewardingParams,
    pub(crate) current_interval: Interval,
    pub(crate) key_rotation_state: KeyRotationState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::caching::cache::test_helpers::round_trip_through_disk_cache;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Timestamp};
    use nym_contracts_common::Percent;
    use nym_mixnet_contract_common::mixnode::PendingMixNodeChanges;
    use nym_mixnet_contract_common::reward_params::{IntervalRewardParams, RewardedSetParams};
    use nym_mixnet_contract_common::NodeCostParams;
    use nym_mixnet_contract_common::{
        Gateway, HistoricalNymNodeVersion, IdentityKey, MixNode, MixNodeBond, NodeRewarding,
        NymNode, NymNodeBond, PendingNodeChanges,
    };
    use std::collections::HashSet;
    use std::time::Duration;

    // Sun Jun 15 2025 15:06:40 GMT+0000
    const DUMMY_TIMESTAMP: u64 = 1750000000;

    fn pledge() -> Coin {
        coin(100_000_000, "unym")
    }

    #[allow(clippy::unwrap_used)]
    fn rewarding_details() -> NodeRewarding {
        let cost_params = NodeCostParams {
            profit_margin_percent: Percent::from_percentage_value(10).unwrap(),
            interval_operating_cost: coin(40_000_000, "unym"),
        };
        NodeRewarding::initialise_new(cost_params, &pledge(), 0).unwrap()
    }

    fn identity(seed: NodeId) -> IdentityKey {
        format!("identity-key-of-node-{seed}")
    }

    fn legacy_mixnode(mix_id: NodeId) -> MixNodeDetails {
        MixNodeDetails::new(
            MixNodeBond {
                mix_id,
                owner: Addr::unchecked(format!("n1owner{mix_id}")),
                original_pledge: pledge(),
                mix_node: MixNode {
                    host: "1.1.1.1".to_string(),
                    mix_port: 1789,
                    verloc_port: 1790,
                    http_api_port: 8000,
                    sphinx_key: format!("sphinx-key-of-node-{mix_id}"),
                    identity_key: identity(mix_id),
                    version: "1.1.5".to_string(),
                },
                proxy: None,
                bonding_height: 100,
                is_unbonding: false,
            },
            rewarding_details(),
            PendingMixNodeChanges::new_empty(),
        )
    }

    fn legacy_gateway(node_id: NodeId) -> CachedLegacyGatewayBond {
        CachedLegacyGatewayBond {
            bond: GatewayBond::new(
                pledge(),
                Addr::unchecked(format!("n1gateway{node_id}")),
                100,
                Gateway {
                    host: "2.2.2.2".to_string(),
                    mix_port: 1789,
                    clients_port: 9000,
                    location: "GB".to_string(),
                    sphinx_key: format!("sphinx-key-of-gateway-{node_id}"),
                    identity_key: identity(node_id),
                    version: "1.1.5".to_string(),
                },
            ),
            node_id,
        }
    }

    fn nym_node(node_id: NodeId) -> NymNodeDetails {
        NymNodeDetails::new(
            NymNodeBond {
                node_id,
                owner: Addr::unchecked(format!("n1node{node_id}")),
                original_pledge: pledge(),
                bonding_height: 100,
                is_unbonding: false,
                node: NymNode {
                    host: "3.3.3.3".to_string(),
                    custom_http_port: Some(8080),
                    identity_key: identity(node_id),
                },
            },
            rewarding_details(),
            PendingNodeChanges::new_empty(),
        )
    }

    fn rewarded_set() -> CachedEpochRewardedSet {
        CachedEpochRewardedSet {
            epoch_id: 5,
            entry_gateways: HashSet::from([1]),
            exit_gateways: HashSet::from([2]),
            layer1: HashSet::from([3]),
            layer2: HashSet::from([4]),
            layer3: HashSet::from([5]),
            standby: HashSet::from([6]),
        }
    }

    fn config_score_data() -> ConfigScoreData {
        ConfigScoreData {
            config_score_params: ConfigScoreParams {
                version_weights: Default::default(),
                version_score_formula_params: Default::default(),
            },
            nym_node_version_history: vec![HistoricalNymNodeVersionEntry {
                id: 0,
                version_information: HistoricalNymNodeVersion {
                    semver: "1.1.5".to_string(),
                    introduced_at_height: 123,
                    difference_since_genesis: Default::default(),
                },
            }],
        }
    }

    #[allow(clippy::unwrap_used)]
    fn reward_params() -> RewardingParams {
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

    fn interval() -> Interval {
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(DUMMY_TIMESTAMP);
        Interval::init_interval(24, Duration::from_secs(60 * 60), &env)
    }

    fn populated_cache() -> MixnetContractCacheData {
        MixnetContractCacheData {
            rewarding_denom: "unym".to_string(),
            legacy_mixnodes: vec![legacy_mixnode(1)],
            legacy_gateways: vec![legacy_gateway(2)],
            nym_nodes: vec![nym_node(3)],
            rewarded_set: rewarded_set(),
            config_score_data: config_score_data(),
            current_reward_params: reward_params(),
            current_interval: interval(),
            key_rotation_state: KeyRotationState {
                validity_epochs: 24,
                initial_epoch_id: 0,
            },
        }
    }

    // This cache is persisted to disk with bincode; a populated value must survive that
    // round trip or the on-disk cache silently never writes.
    #[test]
    fn populated_cache_round_trips_through_the_on_disk_format() -> anyhow::Result<()> {
        let restored = round_trip_through_disk_cache(populated_cache())?;

        assert_eq!(restored.rewarding_denom, "unym");
        assert_eq!(restored.legacy_mixnodes.len(), 1);
        assert_eq!(restored.legacy_gateways.len(), 1);
        assert_eq!(restored.nym_nodes.len(), 1);
        assert_eq!(restored.legacy_gateways[0].node_id, 2);
        assert_eq!(restored.nym_nodes[0].node_id(), 3);
        assert_eq!(restored.rewarded_set.entry_gateways.len(), 1);
        assert_eq!(restored.config_score_data.nym_node_version_history.len(), 1);
        Ok(())
    }
}
