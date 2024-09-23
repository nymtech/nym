// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::helpers::RewardedSetStatus;
use crate::node_status_api::reward_estimate::{compute_apy_from_reward, compute_reward_estimate};
use crate::nym_contract_cache::cache::CachedRewardedSet;
use crate::support::storage::NymApiStorage;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::{
    GatewayBondAnnotated, MixNodeBondAnnotated, NodeAnnotation, NodePerformance,
};
use nym_mixnet_contract_common::{reward_params::Performance, Interval, NodeId};
use nym_mixnet_contract_common::{NymNodeDetails, RewardingParams};
use nym_topology::NetworkAddress;
use std::collections::{HashMap, HashSet};
use std::net::ToSocketAddrs;
use std::str::FromStr;

pub(super) async fn get_mixnode_performance_from_storage(
    storage: &NymApiStorage,
    mix_id: NodeId,
    epoch: Interval,
) -> Option<Performance> {
    storage
        .get_average_mixnode_uptime_in_the_last_24hrs(
            mix_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
        .map(Into::into)
}

pub(super) async fn get_gateway_performance_from_storage(
    storage: &NymApiStorage,
    node_id: NodeId,
    epoch: Interval,
) -> Option<Performance> {
    storage
        .get_average_gateway_uptime_in_the_last_24hrs(
            node_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
        .map(Into::into)
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
            get_mixnode_performance_from_storage(storage, mixnode.mix_id(), current_interval)
                .await
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

pub(crate) async fn annotate_legacy_gateways_with_details(
    storage: &NymApiStorage,
    gateway_bonds: Vec<LegacyGatewayBondWithId>,
    current_interval: Interval,
    blacklist: &HashSet<NodeId>,
) -> HashMap<NodeId, GatewayBondAnnotated> {
    let mut annotated = HashMap::new();
    for gateway_bond in gateway_bonds {
        let performance =
            get_gateway_performance_from_storage(storage, gateway_bond.node_id, current_interval)
                .await
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

pub(crate) async fn produce_node_annotations(
    storage: &NymApiStorage,
    legacy_mixnodes: &[LegacyMixNodeDetailsWithLayer],
    legacy_gateways: &[LegacyGatewayBondWithId],
    nym_nodes: &[NymNodeDetails],
    rewarded_set: &CachedRewardedSet,
    current_interval: Interval,
) -> HashMap<NodeId, NodeAnnotation> {
    let mut annotations = HashMap::new();

    for legacy_mix in legacy_mixnodes {
        let perf = storage
            .get_average_mixnode_uptime_in_the_last_24hrs(
                legacy_mix.mix_id(),
                current_interval.current_epoch_end_unix_timestamp(),
            )
            .await
            .ok()
            .unwrap_or_default()
            .into();

        annotations.insert(
            legacy_mix.mix_id(),
            NodeAnnotation {
                last_24h_performance: perf,
                current_role: rewarded_set.role(legacy_mix.mix_id()),
            },
        );
    }

    for legacy_gateway in legacy_gateways {
        let perf = storage
            .get_average_gateway_uptime_in_the_last_24hrs(
                legacy_gateway.node_id,
                current_interval.current_epoch_end_unix_timestamp(),
            )
            .await
            .ok()
            .unwrap_or_default()
            .into();

        annotations.insert(
            legacy_gateway.node_id,
            NodeAnnotation {
                last_24h_performance: perf,
                current_role: rewarded_set.role(legacy_gateway.node_id),
            },
        );
    }

    for nym_node in nym_nodes {
        let perf = storage
            .get_average_node_uptime_in_the_last_24hrs(
                nym_node.node_id(),
                current_interval.current_epoch_end_unix_timestamp(),
            )
            .await
            .ok()
            .unwrap_or_default()
            .into();

        annotations.insert(
            nym_node.node_id(),
            NodeAnnotation {
                last_24h_performance: perf,
                current_role: rewarded_set.role(nym_node.node_id()),
            },
        );
    }

    annotations
}
