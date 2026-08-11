// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::ed25519;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::nym_mixnet_contract_common::NymNodeBond;
use std::collections::HashMap;
use tokio::sync::{RwLock, RwLockReadGuard};

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

pub(crate) struct BondedNymNodes {
    inner: RwLock<HashMap<NodeId, MinimalNymNode>>,
}

impl BondedNymNodes {
    pub(crate) async fn update(&self, new_nodes: HashMap<NodeId, MinimalNymNode>) {
        *self.inner.write().await = new_nodes;
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, HashMap<NodeId, MinimalNymNode>> {
        self.inner.read().await
    }
}

impl Default for BondedNymNodes {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}
