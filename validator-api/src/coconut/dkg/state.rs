// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dkg::bte::keys::KeyPair;
use dkg::NodeIndex;

pub(crate) struct State {
    keypair: KeyPair,
    node_index: Option<NodeIndex>,
}

impl State {
    pub fn new(keypair: KeyPair) -> Self {
        State {
            keypair,
            node_index: None,
        }
    }

    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }

    pub fn node_index(&self) -> Option<NodeIndex> {
        self.node_index
    }

    pub fn set_node_index(&mut self, node_index: NodeIndex) {
        self.node_index = Some(node_index);
    }
}
