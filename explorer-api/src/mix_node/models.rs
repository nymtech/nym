use std::sync::Arc;
use std::time::SystemTime;

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;

use mixnet_contract::IdentityKey;

use crate::mix_node::cache::Cache;

pub(crate) struct MixNodeCache {
    pub(crate) descriptions: Cache<NodeDescription>,
    pub(crate) node_stats: Cache<NodeStats>,
}

#[derive(Clone)]
pub(crate) struct ThreadsafeMixNodeCache {
    inner: Arc<RwLock<MixNodeCache>>,
}

impl ThreadsafeMixNodeCache {
    pub(crate) fn new() -> Self {
        ThreadsafeMixNodeCache {
            inner: Arc::new(RwLock::new(MixNodeCache {
                descriptions: Cache::new(),
                node_stats: Cache::new(),
            })),
        }
    }

    pub(crate) async fn get_description(
        &self,
        identity_key: IdentityKey,
    ) -> Option<NodeDescription> {
        self.inner.read().await.descriptions.get(identity_key)
    }

    pub(crate) async fn get_node_stats(&self, identity_key: IdentityKey) -> Option<NodeStats> {
        self.inner.read().await.node_stats.get(identity_key)
    }

    pub(crate) async fn set_description(
        &self,
        identity_key: IdentityKey,
        description: NodeDescription,
    ) {
        self.inner
            .write()
            .await
            .descriptions
            .set(identity_key, description);
    }

    pub(crate) async fn set_node_stats(&self, identity_key: IdentityKey, node_stats: NodeStats) {
        self.inner
            .write()
            .await
            .node_stats
            .set(identity_key, node_stats);
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub(crate) struct NodeDescription {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) link: String,
    pub(crate) location: String,
}

#[derive(Serialize, Clone, Deserialize, JsonSchema)]
pub(crate) struct NodeStats {
    #[serde(
        serialize_with = "humantime_serde::serialize",
        deserialize_with = "humantime_serde::deserialize"
    )]
    update_time: SystemTime,

    #[serde(
        serialize_with = "humantime_serde::serialize",
        deserialize_with = "humantime_serde::deserialize"
    )]
    previous_update_time: SystemTime,

    packets_received_since_startup: u64,
    packets_sent_since_startup: u64,
    packets_explicitly_dropped_since_startup: u64,
    packets_received_since_last_update: u64,
    packets_sent_since_last_update: u64,
    packets_explicitly_dropped_since_last_update: u64,
}
