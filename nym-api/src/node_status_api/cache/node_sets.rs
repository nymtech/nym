// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::helpers::RewardedSetStatus;
use crate::node_status_api::models::Uptime;
use crate::node_status_api::reward_estimate::{compute_apy_from_reward, compute_reward_estimate};
use crate::nym_contract_cache::cache::data::ConfigScoreData;
use crate::support::legacy_helpers::legacy_host_to_ips_and_hostname;
use crate::support::storage::NymApiStorage;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::{
    ConfigScore, DescribedNodeType, DetailedNodePerformance, GatewayBondAnnotated,
    MixNodeBondAnnotated, NodeAnnotation, NodePerformance, NymNodeDescription, RoutingScore,
};
use nym_contracts_common::NaiveFloat;
use nym_mixnet_contract_common::{Interval, NodeId, VersionScoreFormulaParams};
use nym_mixnet_contract_common::{NymNodeDetails, RewardingParams};
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use tracing::trace;

pub(super) async fn get_mixnode_reliability_from_storage(
    storage: &NymApiStorage,
    mix_id: NodeId,
    epoch: Interval,
) -> Option<f32> {
    storage
        .get_average_mixnode_reliability_in_the_last_24hrs(
            mix_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
}

pub(super) async fn get_gateway_reliability_from_storage(
    storage: &NymApiStorage,
    node_id: NodeId,
    epoch: Interval,
) -> Option<f32> {
    storage
        .get_average_gateway_reliability_in_the_last_24hrs(
            node_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
}

pub(super) async fn get_node_reliability_from_storage(
    storage: &NymApiStorage,
    node_id: NodeId,
    epoch: Interval,
) -> Option<f32> {
    storage
        .get_average_node_reliability_in_the_last_24hrs(
            node_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
}

async fn get_routing_score(
    storage: &NymApiStorage,
    node_id: NodeId,
    typ: DescribedNodeType,
    epoch: Interval,
) -> RoutingScore {
    let maybe_reliability = match typ {
        DescribedNodeType::LegacyMixnode => {
            get_mixnode_reliability_from_storage(storage, node_id, epoch).await
        }
        DescribedNodeType::LegacyGateway => {
            get_gateway_reliability_from_storage(storage, node_id, epoch).await
        }
        DescribedNodeType::NymNode => {
            get_node_reliability_from_storage(storage, node_id, epoch).await
        }
    };
    // reliability: 0-100
    // score: 0-1
    let reliability = maybe_reliability.unwrap_or_default();
    let score = reliability / 100.;

    trace!("reliability for {node_id}: {maybe_reliability:?}. routing score: {score}");
    RoutingScore::new(score as f64)
}

fn versions_behind_factor_to_config_score(
    versions_behind: u32,
    params: VersionScoreFormulaParams,
) -> f64 {
    let penalty = params.penalty.naive_to_f64();
    let scaling = params.penalty_scaling.naive_to_f64();

    // version_score = penalty ^ (num_versions_behind ^ penalty_scaling)
    penalty.powf((versions_behind as f64).powf(scaling))
}

fn calculate_config_score(
    config_score_data: &ConfigScoreData,
    described_data: Option<&NymNodeDescription>,
) -> ConfigScore {
    let Some(described) = described_data else {
        return ConfigScore::unavailable();
    };

    let node_version = &described.description.build_information.build_version;
    let Ok(reported_semver) = node_version.parse::<semver::Version>() else {
        return ConfigScore::bad_semver();
    };
    let versions_behind = config_score_data
        .config_score_params
        .version_weights
        .versions_behind_factor(
            &reported_semver,
            &config_score_data.nym_node_version_history,
        );

    let runs_nym_node = described.description.build_information.binary_name == "nym-node";
    let accepted_terms_and_conditions = described
        .description
        .auxiliary_details
        .accepted_operator_terms_and_conditions;

    let version_score = if !runs_nym_node || !accepted_terms_and_conditions {
        0.
    } else {
        versions_behind_factor_to_config_score(
            versions_behind,
            config_score_data
                .config_score_params
                .version_score_formula_params,
        )
    };

    ConfigScore::new(
        version_score,
        versions_behind,
        accepted_terms_and_conditions,
        runs_nym_node,
    )
}

// TODO: this might have to be moved to a different file if other places also rely on this functionality
fn get_rewarded_set_status(
    rewarded_set: &CachedEpochRewardedSet,
    node_id: NodeId,
) -> RewardedSetStatus {
    if rewarded_set.is_standby(&node_id) {
        RewardedSetStatus::Standby
    } else if rewarded_set.is_active_mixnode(&node_id) {
        RewardedSetStatus::Active
    } else {
        RewardedSetStatus::Inactive
    }
}

#[deprecated]
pub(super) async fn annotate_legacy_mixnodes_nodes_with_details(
    storage: &NymApiStorage,
    mixnodes: Vec<LegacyMixNodeDetailsWithLayer>,
    interval_reward_params: RewardingParams,
    current_interval: Interval,
    rewarded_set: &CachedEpochRewardedSet,
) -> HashMap<NodeId, MixNodeBondAnnotated> {
    let mut annotated = HashMap::new();
    for mixnode in mixnodes {
        let stake_saturation = mixnode
            .rewarding_details
            .bond_saturation(&interval_reward_params);

        let uncapped_stake_saturation = mixnode
            .rewarding_details
            .uncapped_bond_saturation(&interval_reward_params);

        let rewarded_set_status = get_rewarded_set_status(rewarded_set, mixnode.mix_id());

        // If the performance can't be obtained, because the nym-api was not started with
        // the monitoring (and hence, storage), then reward estimates will be all zero
        let performance =
            get_mixnode_reliability_from_storage(storage, mixnode.mix_id(), current_interval)
                .await
                .map(Uptime::new)
                .map(Into::into)
                .unwrap_or_default();

        let reward_estimate = compute_reward_estimate(
            &mixnode,
            performance,
            rewarded_set_status,
            interval_reward_params,
            current_interval,
        );

        let node_performance = storage
            .construct_mixnode_report(mixnode.mix_id())
            .await
            .map(NodePerformance::from)
            .ok()
            .unwrap_or_default();

        let Some((ip_addresses, _)) =
            legacy_host_to_ips_and_hostname(&mixnode.bond_information.mix_node.host)
        else {
            continue;
        };

        let (estimated_operator_apy, estimated_delegators_apy) =
            compute_apy_from_reward(&mixnode, reward_estimate, current_interval);

        annotated.insert(
            mixnode.mix_id(),
            MixNodeBondAnnotated {
                // all legacy nodes are always blacklisted
                blacklisted: true,
                mixnode_details: mixnode,
                stake_saturation,
                uncapped_stake_saturation,
                performance,
                node_performance,
                estimated_operator_apy,
                estimated_delegators_apy,
                ip_addresses,
            },
        );
    }
    annotated
}

#[deprecated]
pub(crate) async fn annotate_legacy_gateways_with_details(
    storage: &NymApiStorage,
    gateway_bonds: Vec<LegacyGatewayBondWithId>,
    current_interval: Interval,
) -> HashMap<NodeId, GatewayBondAnnotated> {
    let mut annotated = HashMap::new();
    for gateway_bond in gateway_bonds {
        let performance =
            get_gateway_reliability_from_storage(storage, gateway_bond.node_id, current_interval)
                .await
                .map(Uptime::new)
                .map(Into::into)
                .unwrap_or_default();

        let node_performance = storage
            .construct_gateway_report(gateway_bond.node_id)
            .await
            .map(NodePerformance::from)
            .ok()
            .unwrap_or_default();

        let Some((ip_addresses, _)) =
            legacy_host_to_ips_and_hostname(&gateway_bond.bond.gateway.host)
        else {
            continue;
        };

        annotated.insert(
            gateway_bond.node_id,
            GatewayBondAnnotated {
                // all legacy nodes are always blacklisted
                blacklisted: true,
                gateway_bond,
                self_described: None,
                performance,
                node_performance,
                ip_addresses,
            },
        );
    }
    annotated
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn produce_node_annotations(
    storage: &NymApiStorage,
    config_score_data: &ConfigScoreData,
    legacy_mixnodes: &[LegacyMixNodeDetailsWithLayer],
    legacy_gateways: &[LegacyGatewayBondWithId],
    nym_nodes: &[NymNodeDetails],
    rewarded_set: &CachedEpochRewardedSet,
    current_interval: Interval,
    described_nodes: &DescribedNodes,
) -> HashMap<NodeId, NodeAnnotation> {
    let mut annotations = HashMap::new();

    for legacy_mix in legacy_mixnodes {
        let node_id = legacy_mix.mix_id();

        let routing_score = get_routing_score(
            storage,
            node_id,
            DescribedNodeType::LegacyMixnode,
            current_interval,
        )
        .await;
        let config_score =
            calculate_config_score(config_score_data, described_nodes.get_node(&node_id));

        let performance = routing_score.score * config_score.score;
        // map it from 0-1 range into 0-100
        let scaled_performance = performance * 100.;
        let legacy_performance = Uptime::new(scaled_performance as f32).into();

        annotations.insert(
            legacy_mix.mix_id(),
            NodeAnnotation {
                last_24h_performance: legacy_performance,
                current_role: rewarded_set.role(legacy_mix.mix_id()).map(|r| r.into()),
                detailed_performance: DetailedNodePerformance::new(
                    performance,
                    routing_score,
                    config_score,
                ),
            },
        );
    }

    for legacy_gateway in legacy_gateways {
        let node_id = legacy_gateway.node_id;
        let routing_score = get_routing_score(
            storage,
            node_id,
            DescribedNodeType::LegacyGateway,
            current_interval,
        )
        .await;
        let config_score =
            calculate_config_score(config_score_data, described_nodes.get_node(&node_id));

        let performance = routing_score.score * config_score.score;
        // map it from 0-1 range into 0-100
        let scaled_performance = performance * 100.;
        let legacy_performance = Uptime::new(scaled_performance as f32).into();

        annotations.insert(
            legacy_gateway.node_id,
            NodeAnnotation {
                last_24h_performance: legacy_performance,
                current_role: rewarded_set.role(legacy_gateway.node_id).map(|r| r.into()),
                detailed_performance: DetailedNodePerformance::new(
                    performance,
                    routing_score,
                    config_score,
                ),
            },
        );
    }

    for nym_node in nym_nodes {
        let node_id = nym_node.node_id();
        let routing_score = get_routing_score(
            storage,
            node_id,
            DescribedNodeType::NymNode,
            current_interval,
        )
        .await;
        let config_score =
            calculate_config_score(config_score_data, described_nodes.get_node(&node_id));

        let performance = routing_score.score * config_score.score;
        // map it from 0-1 range into 0-100
        let scaled_performance = performance * 100.;
        let legacy_performance = Uptime::new(scaled_performance as f32).into();

        annotations.insert(
            nym_node.node_id(),
            NodeAnnotation {
                last_24h_performance: legacy_performance,
                current_role: rewarded_set.role(nym_node.node_id()).map(|r| r.into()),
                detailed_performance: DetailedNodePerformance::new(
                    performance,
                    routing_score,
                    config_score,
                ),
            },
        );
    }

    annotations
}
