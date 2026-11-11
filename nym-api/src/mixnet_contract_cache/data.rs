// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_api_requests::models::ConfigScoreDataResponse;
use nym_contract_anchor::whole_set;
use nym_crypto::asymmetric::ed25519;
use nym_mixnet_contract_common::{
    ConfigScoreParams, HistoricalNymNodeVersionEntry, Interval, KeyRotationState, NodeId,
    NymNodeDetails, RewardingParams,
};
use nym_topology::CachedEpochRewardedSet;
use nym_validator_client::nyxd::contract_traits::PinnedMixnetQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::block::Height;

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

/// The mixnet contract at a snapshot height, read unproven beside the attested snapshots.
#[derive(Serialize, Deserialize)]
pub(crate) struct MixnetContractData {
    pub(crate) rewarding_denom: String,
    pub(crate) nym_nodes: Vec<NymNodeDetails>,
    pub(crate) rewarded_set: CachedEpochRewardedSet,
    pub(crate) config_score_data: ConfigScoreData,
    pub(crate) current_reward_params: RewardingParams,
    pub(crate) current_interval: Interval,
    pub(crate) key_rotation_state: KeyRotationState,
}

impl MixnetContractData {
    /// Every value as of `height`, each query pinned there.
    pub(crate) async fn read<C>(client: &C, height: Height) -> Result<Self, NyxdError>
    where
        C: PinnedMixnetQueryClient + Sync,
    {
        let contract_state = client.get_mixnet_contract_state_at_height(height).await?;
        Ok(MixnetContractData {
            rewarding_denom: contract_state.rewarding_denom,
            nym_nodes: client.get_all_nymnodes_detailed_at_height(height).await?,
            rewarded_set: client.get_rewarded_set_at_height(height).await?.into(),
            config_score_data: ConfigScoreData {
                config_score_params: contract_state.params.config_score_params,
                nym_node_version_history: client
                    .get_full_nym_node_version_history_at_height(height)
                    .await?,
            },
            current_reward_params: client.get_rewarding_parameters_at_height(height).await?,
            current_interval: client
                .get_current_interval_details_at_height(height)
                .await?
                .interval,
            key_rotation_state: client.get_key_rotation_state_at_height(height).await?,
        })
    }

    /// The identity key of every bonded node, the map every attested contract is verified with.
    pub(crate) fn node_identities(&self) -> BTreeMap<NodeId, ed25519::PublicKey> {
        whole_set::node_identities(self.nym_nodes.iter().map(|node| &node.bond_information))
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::{coin, Addr, Coin, Decimal, Timestamp};
    use nym_contracts_common::Percent;
    use nym_mixnet_contract_common::reward_params::{IntervalRewardParams, RewardedSetParams};
    use nym_mixnet_contract_common::NodeCostParams;
    use nym_mixnet_contract_common::{
        HistoricalNymNodeVersion, IdentityKey, NodeRewarding, NymNode, NymNodeBond,
        PendingNodeChanges,
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

    /// Mixnet contract data holding one node and every other field set.
    pub(crate) fn populated_mixnet_data() -> MixnetContractData {
        MixnetContractData {
            rewarding_denom: "unym".to_string(),
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
}
