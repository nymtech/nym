use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;

pub(crate) type PingCache = HashMap<String, PingCacheItem>;

const CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour

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

    pub(crate) async fn get(&self, identity_key: &str) -> Option<PingResponse> {
        self.inner
            .read()
            .await
            .get(identity_key)
            .filter(|cache_item| cache_item.valid_until > SystemTime::now())
            .map(|cache_item| PingResponse {
                ports: cache_item.ports.clone(),
            })
    }

    pub(crate) async fn set(&self, identity_key: &str, item: PingResponse) {
        self.inner.write().await.insert(
            identity_key.to_string(),
            PingCacheItem {
                valid_until: SystemTime::now() + CACHE_TTL,
                ports: item.ports,
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
