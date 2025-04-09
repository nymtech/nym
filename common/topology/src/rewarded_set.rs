// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::{EpochId, EpochRewardedSet, NodeId, RewardedSet};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CachedEpochRewardedSet {
    pub epoch_id: EpochId,

    pub entry_gateways: HashSet<NodeId>,

    pub exit_gateways: HashSet<NodeId>,

    pub layer1: HashSet<NodeId>,

    pub layer2: HashSet<NodeId>,

    pub layer3: HashSet<NodeId>,

    pub standby: HashSet<NodeId>,
}

impl From<EpochRewardedSet> for CachedEpochRewardedSet {
    fn from(value: EpochRewardedSet) -> Self {
        CachedEpochRewardedSet {
            epoch_id: value.epoch_id,
            entry_gateways: value.assignment.entry_gateways.into_iter().collect(),
            exit_gateways: value.assignment.exit_gateways.into_iter().collect(),
            layer1: value.assignment.layer1.into_iter().collect(),
            layer2: value.assignment.layer2.into_iter().collect(),
            layer3: value.assignment.layer3.into_iter().collect(),
            standby: value.assignment.standby.into_iter().collect(),
        }
    }
}

impl From<CachedEpochRewardedSet> for EpochRewardedSet {
    fn from(value: CachedEpochRewardedSet) -> Self {
        EpochRewardedSet {
            epoch_id: value.epoch_id,
            assignment: RewardedSet {
                entry_gateways: value.entry_gateways.into_iter().collect(),
                exit_gateways: value.exit_gateways.into_iter().collect(),
                layer1: value.layer1.into_iter().collect(),
                layer2: value.layer2.into_iter().collect(),
                layer3: value.layer3.into_iter().collect(),
                standby: value.standby.into_iter().collect(),
            },
        }
    }
}

impl CachedEpochRewardedSet {
    pub fn is_empty(&self) -> bool {
        self.entry_gateways.is_empty()
            && self.exit_gateways.is_empty()
            && self.layer1.is_empty()
            && self.layer2.is_empty()
            && self.layer3.is_empty()
            && self.standby.is_empty()
    }

    pub fn role(&self, node_id: NodeId) -> Option<Role> {
        if self.entry_gateways.contains(&node_id) {
            Some(Role::EntryGateway)
        } else if self.exit_gateways.contains(&node_id) {
            Some(Role::ExitGateway)
        } else if self.layer1.contains(&node_id) {
            Some(Role::Layer1)
        } else if self.layer2.contains(&node_id) {
            Some(Role::Layer2)
        } else if self.layer3.contains(&node_id) {
            Some(Role::Layer3)
        } else if self.standby.contains(&node_id) {
            Some(Role::Standby)
        } else {
            None
        }
    }

    pub fn legacy_mix_layer(&self, node_id: &NodeId) -> Option<u8> {
        if self.layer1.contains(node_id) {
            Some(1)
        } else if self.layer2.contains(node_id) {
            Some(2)
        } else if self.layer3.contains(node_id) {
            Some(3)
        } else {
            None
        }
    }

    pub fn is_standby(&self, node_id: &NodeId) -> bool {
        self.standby.contains(node_id)
    }

    pub fn is_active_mixnode(&self, node_id: &NodeId) -> bool {
        self.layer1.contains(node_id)
            || self.layer2.contains(node_id)
            || self.layer3.contains(node_id)
    }

    pub fn gateways(&self) -> HashSet<NodeId> {
        let mut gateways =
            HashSet::with_capacity(self.entry_gateways.len() + self.exit_gateways.len());
        gateways.extend(&self.entry_gateways);
        gateways.extend(&self.exit_gateways);
        gateways
    }

    pub fn active_mixnodes(&self) -> HashSet<NodeId> {
        let mut mixnodes =
            HashSet::with_capacity(self.layer1.len() + self.layer2.len() + self.layer3.len());
        mixnodes.extend(&self.layer1);
        mixnodes.extend(&self.layer2);
        mixnodes.extend(&self.layer3);
        mixnodes
    }

    pub fn all_ids(&self) -> HashSet<NodeId> {
        let mut mixnodes = HashSet::with_capacity(
            self.entry_gateways.len()
                + self.exit_gateways.len()
                + self.layer1.len()
                + self.layer2.len()
                + self.layer3.len()
                + self.standby.len(),
        );
        mixnodes.extend(&self.entry_gateways);
        mixnodes.extend(&self.exit_gateways);
        mixnodes.extend(&self.layer1);
        mixnodes.extend(&self.layer2);
        mixnodes.extend(&self.layer3);
        mixnodes.extend(&self.standby);
        mixnodes
    }
}
