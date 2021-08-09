use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use mixnet_contract::IdentityKey;
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub(crate) struct Cache<T> {
    inner: HashMap<IdentityKey, CacheItem<T>>,
}

impl<T> Cache<T> {
    pub(crate) fn new() -> Self {
        Cache {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, identity_key: IdentityKey) -> Option<T>
    where
        T: Clone,
    {
        self.inner.get(&identity_key).and_then(|cache_item| {
            if cache_item.valid_until > SystemTime::now() {
                Some(cache_item.clone().value)
            } else {
                None
            }
        })
    }

    pub(crate) fn set(&mut self, identity_key: IdentityKey, value: T) {
        self.inner.insert(
            identity_key,
            CacheItem {
                valid_until: SystemTime::now() + Duration::from_secs(5),
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
