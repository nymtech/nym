use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;

use mixnet_contract::IdentityKey;

pub(crate) type PingCache = HashMap<IdentityKey, PingCacheItem>;

#[derive(Clone)]
pub(crate) struct ThreadsafePingCache {
    inner: Arc<RwLock<PingCache>>,
}

impl ThreadsafePingCache {
    pub(crate) fn new() -> Self {
        ThreadsafePingCache {
            inner: Arc::new(RwLock::new(PingCache::new())),
        }
    }

    pub(crate) async fn get(self, identity_key: IdentityKey) -> Option<PingResponse> {
        self.inner
            .read()
            .await
            .get(&identity_key)
            .and_then(|cache_item| {
                if cache_item.valid_until.gt(&SystemTime::now()) {
                    // return if cache item is still valid
                    Some(PingResponse {
                        ports: cache_item.ports.clone(),
                    })
                } else {
                    None
                }
            })
    }

    pub(crate) async fn set(self, identity_key: IdentityKey, item: PingResponse) {
        self.inner.write().await.insert(
            identity_key,
            PingCacheItem {
                valid_until: SystemTime::now() + Duration::from_secs(5),
                ports: item.ports.clone(),
            },
        );
    }
}

#[derive(Deserialize, Serialize, JsonSchema, Clone)]
pub(crate) struct PingResponse {
    pub(crate) ports: HashMap<u16, bool>,
}

pub(crate) struct PingCacheItem {
    pub(crate) ports: HashMap<u16, bool>,
    pub(crate) valid_until: std::time::SystemTime,
}
