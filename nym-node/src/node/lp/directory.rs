// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use arc_swap::ArcSwap;
use nym_lp::peer::{DHPublicKey, LpRemotePeer};
use nym_lp::{KEM, KEMKeyDigests};
use nym_lp_data::packet::version;
use nym_topology::NodeId;
use nym_validator_client::models::described::type_translation::{
    LewesProtocolDetailsDataV1, MalformedLPData,
};
use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

/// Wrapper around all known LP nodes
#[derive(Clone)]
pub struct LpNodes {
    // map between all available ip addresses of other nodes and their details
    inner: Arc<LpNodesInner>,
}

impl LpNodes {
    pub fn new(nodes: HashMap<IpAddr, LpNodeDetails>) -> Self {
        // ensure we're always storing canonical IPs
        LpNodes {
            inner: Arc::new(LpNodesInner {
                update_lock: Mutex::new(()),
                nodes: ArcSwap::from_pointee(
                    nodes
                        .into_iter()
                        .map(|(k, v)| (k.to_canonical(), v))
                        .collect(),
                ),
            }),
        }
    }

    pub fn new_empty() -> Self {
        Self::new(Default::default())
    }

    pub(crate) fn get_node_details(&self, node_ip: IpAddr) -> Option<LpNodeDetails> {
        self.inner
            .nodes
            .load()
            .get(&node_ip.to_canonical())
            .cloned()
    }

    pub async fn get_update_permit(&self) -> MutexGuard<'_, ()> {
        self.inner.update_lock.lock().await
    }

    /// Atomically replace the lp nodes map.
    ///
    /// # Precondition
    ///
    /// The caller **must** hold the permit returned by [`LpNodes::get_update_permit`].
    /// Passing the `MutexGuard` by value enforces this at the type level — the guard is dropped
    /// (releasing the lock) only after the swap completes, preventing torn writes from concurrent
    /// update calls.
    pub fn swap_view(&self, _permit: MutexGuard<'_, ()>, new: HashMap<IpAddr, LpNodeDetails>) {
        // defensive: ensure stored keys are always canonical so lookups (which canonicalise)
        // always match. callers should still canonicalise before assembling `new` to keep
        // collision resolution deterministic.
        let canonical = new
            .into_iter()
            .map(|(k, v)| (k.to_canonical(), v))
            .collect();
        self.inner.nodes.store(Arc::new(canonical));
    }
}

/// Inner state of [`LpNodes`], shared behind an `Arc`.
///
/// # Concurrency model
///
/// Reads (on the packet-processing hot path) use `ArcSwap` and are fully lock-free.
/// Writers must first acquire `update_lock` to serialise concurrent updates, then call
/// `swap_view` to atomically publish the new map.  The lock is intentionally *not* wrapping
/// the map itself so that readers are never blocked.
struct LpNodesInner {
    update_lock: Mutex<()>,
    // map between all available ip addresses of other nodes and their details
    nodes: ArcSwap<HashMap<IpAddr, LpNodeDetails>>,
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
        details: LewesProtocolDetailsDataV1,
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
