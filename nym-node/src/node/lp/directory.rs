// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::ArcSwap;
use nym_lp::peer::{DHPublicKey, LpRemotePeer};
use nym_lp::{KEM, KEMKeyDigests};
use nym_lp_data::packet::version;
use nym_topology::NodeId;
use nym_validator_client::models::described::type_translation::{
    LewesProtocolDetailsDataV2, MalformedLPData,
};
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
    pub fn new(nodes: HashMap<IpAddr, LpNodeDetails>) -> Self {
        let lp_nodes = Self::new_empty();
        lp_nodes.swap_view(nodes);
        lp_nodes
    }

    pub fn new_empty() -> Self {
        Self::default()
    }

    pub(crate) fn get_node_details(&self, node_ip: IpAddr) -> Option<LpNodeDetails> {
        self.nodes.load().get(&node_ip.to_canonical()).cloned()
    }

    /// Atomically replace the lp nodes map.
    ///
    pub fn swap_view(&self, new: HashMap<IpAddr, LpNodeDetails>) {
        // defensive: ensure stored keys are always canonical so lookups (which canonicalise)
        // always match. callers should still canonicalise before assembling `new` to keep
        // collision resolution deterministic.
        let canonical = new
            .into_iter()
            .map(|(k, v)| (k.to_canonical(), v))
            .collect();
        self.nodes.store(Arc::new(canonical));
    }
}

#[derive(Clone)]
pub struct LpNodeDetails {
    inner: Arc<LpNodeDetailsInner>,
}

impl LpNodeDetails {
    pub fn new(
        node_id: NodeId,
        kem_key_hashes: BTreeMap<KEM, KEMKeyDigests>,
        x25519: DHPublicKey,
        control_port: u16,
        data_port: u16,
        supported_protocol: u8,
    ) -> Self {
        LpNodeDetails {
            inner: Arc::new(LpNodeDetailsInner {
                node_id,
                control_port,
                data_port,
                kem_key_hashes,
                x25519,
                supported_protocol,
            }),
        }
    }

    pub fn try_from_details_data(
        details: LewesProtocolDetailsDataV2,
        node_id: NodeId,
    ) -> Result<Self, MalformedLPData> {
        let kem_key_hashes = details.kem_keys()?;
        Ok(Self::new(
            node_id,
            kem_key_hashes,
            details.x25519,
            details.control_port,
            details.data_port,
            version::CURRENT,
        ))
    }
}

impl Deref for LpNodeDetails {
    type Target = LpNodeDetailsInner;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

pub struct LpNodeDetailsInner {
    pub node_id: NodeId,
    pub control_port: u16,
    pub data_port: u16,
    pub kem_key_hashes: BTreeMap<KEM, KEMKeyDigests>,
    pub x25519: DHPublicKey,
    pub supported_protocol: u8,
}

impl LpNodeDetailsInner {
    pub(crate) fn to_lp_peer(&self) -> LpRemotePeer {
        LpRemotePeer::new(self.x25519).with_key_digests(self.kem_key_hashes.clone())
    }
}
