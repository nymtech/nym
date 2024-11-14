// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::helpers::RewardedSetStatus;
use crate::node_status_api::models::Uptime;
use crate::node_status_api::reward_estimate::{compute_apy_from_reward, compute_reward_estimate};
use crate::nym_contract_cache::cache::CachedRewardedSet;
use crate::support::storage::NymApiStorage;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::DescribedNodeType::{LegacyGateway, LegacyMixnode, NymNode};
use nym_api_requests::models::{
    ConfigScore, DescribedNodeType, DetailedNodePerformance, GatewayBondAnnotated,
    MixNodeBondAnnotated, NodeAnnotation, NodePerformance, NymNodeDescription, RoutingScore,
};
use nym_contracts_common::NaiveFloat;
use nym_mixnet_contract_common::{ConfigScoreParams, Interval, NodeId};
use nym_mixnet_contract_common::{NymNodeDetails, RewardingParams};
use nym_topology::NetworkAddress;
use std::collections::{HashMap, HashSet};
use std::net::ToSocketAddrs;
use std::str::FromStr;
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

fn calculate_config_score(
    config_score_params: &ConfigScoreParams,
    described_data: Option<&NymNodeDescription>,
) -> ConfigScore {
    let Some(described) = described_data else {
        return ConfigScore::unavailable();
    };

    let Ok(reported_semver) = described
        .description
        .build_information
        .build_version
        .parse::<semver::Version>()
    else {
        return ConfigScore::bad_semver();
    };

    let versions_behind = config_score_params.versions_behind(&reported_semver);
    let runs_nym_node = described.description.build_information.binary_name == "nym-node";
    let accepted_terms_and_conditions = described
        .description
        .auxiliary_details
        .accepted_operator_terms_and_conditions;

    let version_score = if !runs_nym_node || !accepted_terms_and_conditions {
        0.
    } else {
        let penalty = config_score_params
            .version_score_formula_params
            .penalty
            .naive_to_f64();
        let scaling = config_score_params
            .version_score_formula_params
            .penalty_scaling
            .naive_to_f64();

        // version_score = penalty ^ (num_versions_behind ^ penalty_scaling)
        penalty.powf((versions_behind as f64).powf(scaling))
    };

    ConfigScore::new(
        version_score,
        versions_behind,
        accepted_terms_and_conditions,
        runs_nym_node,
    )
}

// TODO: this might have to be moved to a different file if other places also rely on this functionality
fn get_rewarded_set_status(rewarded_set: &CachedRewardedSet, node_id: NodeId) -> RewardedSetStatus {
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
    rewarded_set: &CachedRewardedSet,
    blacklist: &HashSet<NodeId>,
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

        // safety: this conversion is infallible
        let ip_addresses =
            match NetworkAddress::from_str(&mixnode.bond_information.mix_node.host).unwrap() {
                NetworkAddress::IpAddr(ip) => vec![ip],
                NetworkAddress::Hostname(hostname) => {
                    // try to resolve it
                    (
                        hostname.as_str(),
                        mixnode.bond_information.mix_node.mix_port,
                    )
                        .to_socket_addrs()
                        .map(|iter| iter.map(|s| s.ip()).collect::<Vec<_>>())
                        .unwrap_or_default()
                }
            };

        let (estimated_operator_apy, estimated_delegators_apy) =
            compute_apy_from_reward(&mixnode, reward_estimate, current_interval);

        annotated.insert(
            mixnode.mix_id(),
            MixNodeBondAnnotated {
                blacklisted: blacklist.contains(&mixnode.mix_id()),
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
    blacklist: &HashSet<NodeId>,
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

        // safety: this conversion is infallible
        let ip_addresses = match NetworkAddress::from_str(&gateway_bond.bond.gateway.host).unwrap()
        {
            NetworkAddress::IpAddr(ip) => vec![ip],
            NetworkAddress::Hostname(hostname) => {
                // try to resolve it
                (hostname.as_str(), gateway_bond.bond.gateway.mix_port)
                    .to_socket_addrs()
                    .map(|iter| iter.map(|s| s.ip()).collect::<Vec<_>>())
                    .unwrap_or_default()
            }
        };

        annotated.insert(
            gateway_bond.node_id,
            GatewayBondAnnotated {
                blacklisted: blacklist.contains(&gateway_bond.node_id),
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
    config_score_params: &ConfigScoreParams,
    legacy_mixnodes: &[LegacyMixNodeDetailsWithLayer],
    legacy_gateways: &[LegacyGatewayBondWithId],
    nym_nodes: &[NymNodeDetails],
    rewarded_set: &CachedRewardedSet,
    current_interval: Interval,
    described_nodes: &DescribedNodes,
) -> HashMap<NodeId, NodeAnnotation> {
    let mut annotations = HashMap::new();

    for legacy_mix in legacy_mixnodes {
        let node_id = legacy_mix.mix_id();

        let routing_score =
            get_routing_score(storage, node_id, LegacyMixnode, current_interval).await;
        let config_score =
            calculate_config_score(config_score_params, described_nodes.get_node(&node_id));

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
        let routing_score =
            get_routing_score(storage, node_id, LegacyGateway, current_interval).await;
        let config_score =
            calculate_config_score(config_score_params, described_nodes.get_node(&node_id));

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
        let routing_score = get_routing_score(storage, node_id, NymNode, current_interval).await;
        let config_score =
            calculate_config_score(config_score_params, described_nodes.get_node(&node_id));

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
