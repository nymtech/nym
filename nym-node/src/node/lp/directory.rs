// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::ArcSwap;
use nym_lp::peer::{DHPublicKey, LpRemotePeer};
use nym_lp::{KEM, KEMKeyDigests};
use nym_topology::NodeId;
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::ops::Deref;
use std::sync::Arc;

/// Wrapper around all known LP nodes
#[derive(Clone, Default)]
pub struct LpNodes {
    // map between all available ip addresses of other nodes and their details
    nodes: Arc<ArcSwap<HashMap<IpAddr, LpNodeDetails>>>,
}

impl LpNodes {
    pub(crate) fn is_from_known_node(&self, node_ip: IpAddr) -> bool {
        self.nodes.load().contains_key(&node_ip)
    }

    pub(crate) fn get_node_details(&self, node_ip: IpAddr) -> Option<LpNodeDetails> {
        self.nodes.load().get(&node_ip).cloned()
    }
}

#[derive(Clone)]
pub(crate) struct LpNodeDetails {
    inner: Arc<LpNodeDetailsInner>,
}

impl LpNodeDetails {
    pub(crate) fn new(
        node_id: NodeId,
        kem_key_hashes: BTreeMap<KEM, KEMKeyDigests>,
        x25519: DHPublicKey,
        supported_protocol: u8,
    ) -> Self {
        LpNodeDetails {
            inner: Arc::new(LpNodeDetailsInner {
                node_id,
                kem_key_hashes,
                x25519,
                supported_protocol,
            }),
        }
    }
}

impl Deref for LpNodeDetails {
    type Target = LpNodeDetailsInner;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

pub(crate) struct LpNodeDetailsInner {
    pub(crate) node_id: NodeId,
    pub(crate) kem_key_hashes: BTreeMap<KEM, KEMKeyDigests>,
    pub(crate) x25519: DHPublicKey,
    pub(crate) supported_protocol: u8,
}

impl LpNodeDetailsInner {
    pub(crate) fn to_lp_peer(&self) -> LpRemotePeer {
        LpRemotePeer::new(self.x25519).with_key_digests(self.kem_key_hashes.clone())
    }
}
