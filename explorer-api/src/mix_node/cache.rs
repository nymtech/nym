use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use mixnet_contract::IdentityKey;

#[derive(Clone)]
pub(crate) struct Cache<T: Clone> {
    inner: HashMap<IdentityKey, CacheItem<T>>,
}

impl<T: Clone> Cache<T> {
    pub(crate) fn new() -> Self {
        Cache {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, identity_key: IdentityKey) -> Option<T>
    where
        T: Clone,
    {
        self.inner
            .get(&identity_key)
            .filter(|cache_item| cache_item.valid_until > SystemTime::now())
            .map(|cache_item| cache_item.value.clone())
    }

    pub(crate) fn set(&mut self, identity_key: IdentityKey, value: T) {
        self.inner.insert(
            identity_key,
            CacheItem {
                valid_until: SystemTime::now() + Duration::from_secs(60 * 30),
                value,
            },
        );
    }
}

#[derive(Clone)]
pub(crate) struct CacheItem<T> {
    pub(crate) value: T,
    pub(crate) valid_until: std::time::SystemTime,
}
