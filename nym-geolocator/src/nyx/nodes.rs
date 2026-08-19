// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, PagedMixnetQueryClient};
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};
use tracing::error;

#[derive(Clone)]
pub(crate) struct MinimalNymNode {
    pub(crate) host: String,
    pub(crate) custom_http_port: Option<u16>,
    pub(crate) node_id: NodeId,
    pub(crate) identity: ed25519::PublicKey,
}

impl TryFrom<NymNodeBond> for MinimalNymNode {
    type Error = ed25519::Ed25519RecoveryError;

    fn try_from(value: NymNodeBond) -> Result<Self, Self::Error> {
        Ok(MinimalNymNode {
            host: value.node.host,
            custom_http_port: value.node.custom_http_port,
            node_id: value.node_id,
            identity: ed25519::PublicKey::from_base58_string(&value.node.identity_key)?,
        })
    }
}

/// The bonded set as of the last chain refresh, shared between the watcher that writes it and
/// the scraper that reads it - hence the `Arc`, matching [`crate::nyx::state::OnChainNodes`].
#[derive(Clone, Default)]
pub(crate) struct BondedNymNodes {
    inner: Arc<RwLock<HashMap<NodeId, MinimalNymNode>>>,
}

pub(crate) async fn get_bonded_nodes<C>(
    client: &C,
) -> anyhow::Result<HashMap<NodeId, MinimalNymNode>>
where
    C: MixnetQueryClient + Send + Sync,
{
    let nodes = client.get_all_nymnode_bonds().await?;
    let mut bonded_nodes = HashMap::new();
    for node in nodes {
        if !node.is_unbonding {
            let node_id = node.node_id;
            match MinimalNymNode::try_from(node) {
                Ok(node) => {
                    bonded_nodes.insert(node_id, node);
                }
                Err(err) => {
                    error!("node {node_id} has announced malformed identity key: {err}",);
                }
            }
        }
    }
    Ok(bonded_nodes)
}

impl BondedNymNodes {
    pub(crate) async fn build_new<C>(client: &C) -> anyhow::Result<Self>
    where
        C: MixnetQueryClient + Send + Sync,
    {
        Ok(BondedNymNodes {
            inner: Arc::new(RwLock::new(get_bonded_nodes(client).await?)),
        })
    }

    pub(crate) async fn update(&self, new_nodes: HashMap<NodeId, MinimalNymNode>) {
        *self.inner.write().await = new_nodes;
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, HashMap<NodeId, MinimalNymNode>> {
        self.inner.read().await
    }

    /// The ids currently bonded and not unbonding.
    pub(crate) async fn known_ids(&self) -> HashSet<NodeId> {
        self.inner.read().await.keys().copied().collect()
    }
}
