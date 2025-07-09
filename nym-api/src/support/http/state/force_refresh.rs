// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_mixnet_contract_common::NodeId;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;

#[derive(Clone)]
pub(crate) struct ForcedRefresh {
    pub(crate) allow_all_ip_addresses: bool,
    pub(crate) refreshes: Arc<RwLock<HashMap<NodeId, OffsetDateTime>>>,
}

impl ForcedRefresh {
    pub(crate) fn new(allow_all_ip_addresses: bool) -> ForcedRefresh {
        ForcedRefresh {
            allow_all_ip_addresses,
            refreshes: Arc::new(Default::default()),
        }
    }

    pub(crate) async fn last_refreshed(&self, node_id: NodeId) -> Option<OffsetDateTime> {
        self.refreshes.read().await.get(&node_id).copied()
    }

    pub(crate) async fn set_last_refreshed(&self, node_id: NodeId) {
        self.refreshes
            .write()
            .await
            .insert(node_id, OffsetDateTime::now_utc());
    }
}
