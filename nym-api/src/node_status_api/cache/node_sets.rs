// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::reward_estimate::{compute_apy_from_reward, compute_reward_estimate};
use crate::support::storage::NymApiStorage;
use nym_api_requests::models::{GatewayBondAnnotated, MixNodeBondAnnotated, NodePerformance};
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::{reward_params::Performance, Interval, MixId};
use nym_mixnet_contract_common::{
    GatewayBond, IdentityKey, MixNodeDetails, RewardedSetNodeStatus, RewardingParams,
};
use std::collections::{HashMap, HashSet};

pub(super) fn to_rewarded_set_node_status(
    rewarded_set: &[MixNodeDetails],
    active_set: &[MixNodeDetails],
) -> HashMap<MixId, RewardedSetNodeStatus> {
    let mut rewarded_set_node_status: HashMap<MixId, RewardedSetNodeStatus> = rewarded_set
        .iter()
        .map(|m| (m.mix_id(), RewardedSetNodeStatus::Standby))
        .collect();
    for mixnode in active_set {
        *rewarded_set_node_status
            .get_mut(&mixnode.mix_id())
            .expect("All active nodes are rewarded nodes") = RewardedSetNodeStatus::Active;
    }
    rewarded_set_node_status
}

pub(super) fn split_into_active_and_rewarded_set(
    mixnodes_annotated: &HashMap<MixId, MixNodeBondAnnotated>,
    rewarded_set_node_status: &HashMap<u32, RewardedSetNodeStatus>,
) -> (Vec<MixNodeBondAnnotated>, Vec<MixNodeBondAnnotated>) {
    let rewarded_set: Vec<_> = mixnodes_annotated
        .values()
        .filter(|mixnode| rewarded_set_node_status.get(&mixnode.mix_id()).is_some())
        .cloned()
        .collect();
    let active_set: Vec<_> = rewarded_set
        .iter()
        .filter(|mixnode| {
            rewarded_set_node_status
                .get(&mixnode.mix_id())
                .map_or(false, RewardedSetNodeStatus::is_active)
        })
        .cloned()
        .collect();
    (rewarded_set, active_set)
}

pub(super) async fn get_mixnode_performance_from_storage(
    storage: &Option<NymApiStorage>,
    mix_id: MixId,
    epoch: Interval,
) -> Option<Performance> {
    storage
        .as_ref()?
        .get_average_mixnode_uptime_in_the_last_24hrs(
            mix_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
        .map(Into::into)
}

pub(super) async fn get_gateway_performance_from_storage(
    storage: &Option<NymApiStorage>,
    gateway_id: &str,
    epoch: Interval,
) -> Option<Performance> {
    storage
        .as_ref()?
        .get_average_gateway_uptime_in_the_last_24hrs(
            gateway_id,
            epoch.current_epoch_end_unix_timestamp(),
        )
        .await
        .ok()
        .map(Into::into)
}

pub(super) async fn annotate_nodes_with_details(
    storage: &Option<NymApiStorage>,
    mixnodes: Vec<MixNodeDetails>,
    interval_reward_params: RewardingParams,
    current_interval: Interval,
    rewarded_set: &HashMap<MixId, RewardedSetNodeStatus>,
    mix_to_family: Vec<(IdentityKey, FamilyHead)>,
    blacklist: &HashSet<MixId>,
) -> HashMap<MixId, MixNodeBondAnnotated> {
    let mix_to_family = mix_to_family
        .into_iter()
        .collect::<HashMap<IdentityKey, FamilyHead>>();

    let mut annotated = HashMap::new();
    for mixnode in mixnodes {
        let stake_saturation = mixnode
            .rewarding_details
            .bond_saturation(&interval_reward_params);

        let uncapped_stake_saturation = mixnode
            .rewarding_details
            .uncapped_bond_saturation(&interval_reward_params);

        let rewarded_set_status = rewarded_set.get(&mixnode.mix_id()).copied();

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

        let node_performance = if let Some(storage) = storage {
            storage
                .construct_mixnode_report(mixnode.mix_id())
                .await
                .map(NodePerformance::from)
                .ok()
        } else {
            None
        }
        .unwrap_or_default();

        let (estimated_operator_apy, estimated_delegators_apy) =
            compute_apy_from_reward(&mixnode, reward_estimate, current_interval);

        let family = mix_to_family
            .get(mixnode.bond_information.identity())
            .cloned();

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
                family,
            },
        );
    }
    annotated
}

pub(crate) async fn annotate_gateways_with_details(
    storage: &Option<NymApiStorage>,
    gateway_bonds: Vec<GatewayBond>,
    current_interval: Interval,
    blacklist: &HashSet<IdentityKey>,
) -> HashMap<IdentityKey, GatewayBondAnnotated> {
    let mut annotated = HashMap::new();
    for gateway_bond in gateway_bonds {
        let performance = get_gateway_performance_from_storage(
            storage,
            gateway_bond.identity(),
            current_interval,
        )
        .await
        .unwrap_or_default();

        let node_performance = if let Some(storage) = storage {
            storage
                .construct_gateway_report(gateway_bond.identity())
                .await
                .map(NodePerformance::from)
                .ok()
        } else {
            None
        }
        .unwrap_or_default();

        annotated.insert(
            gateway_bond.identity().to_string(),
            GatewayBondAnnotated {
                blacklisted: blacklist.contains(&gateway_bond.gateway.identity_key),
                gateway_bond,
                self_described: None,
                performance,
                node_performance,
            },
        );
    }
    annotated
}
