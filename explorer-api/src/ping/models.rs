use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;

pub(crate) type PingCache = HashMap<String, PingCacheItem>;

const PING_TTL: Duration = Duration::from_secs(60 * 5); // 5 mins, before port check will be re-tried (only while pending)
const CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour, to cache result from port check

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
            .map(|cache_item| {
                if cache_item.pending {
                    return PingResponse {
                        pending: true,
                        ports: None,
                    };
                }
                PingResponse {
                    pending: false,
                    ports: cache_item.ports.clone(),
                }
            })
    }

    pub(crate) async fn set_pending(&self, identity_key: &str) {
        self.inner.write().await.insert(
            identity_key.to_string(),
            PingCacheItem {
                pending: true,
                valid_until: SystemTime::now() + PING_TTL,
                ports: None,
            },
        );
    }

    pub(crate) async fn set(&self, identity_key: &str, item: PingResponse) {
        self.inner.write().await.insert(
            identity_key.to_string(),
            PingCacheItem {
                pending: false,
                valid_until: SystemTime::now() + CACHE_TTL,
                ports: item.ports,
            },
        );
    }
}

#[derive(Deserialize, Serialize, JsonSchema, Clone)]
pub(crate) struct PingResponse {
    pub(crate) pending: bool,
    pub(crate) ports: Option<HashMap<u16, bool>>,
}

pub(crate) struct PingCacheItem {
    pub(crate) pending: bool,
    pub(crate) ports: Option<HashMap<u16, bool>>,
    pub(crate) valid_until: std::time::SystemTime,
}
