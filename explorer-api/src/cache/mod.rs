use std::collections::HashMap;
use std::time::{Duration, SystemTime};

#[derive(Clone)]
pub(crate) struct Cache<T: Clone> {
    inner: HashMap<String, CacheItem<T>>,
}

impl<T: Clone> Cache<T> {
    pub(crate) fn new() -> Self {
        Cache {
            inner: HashMap::new(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn get_all(&self) -> Vec<T> {
        self.inner
            .values()
            .map(|cache_item| cache_item.value.clone())
            .collect()
    }

    pub(crate) fn get(&self, key: &str) -> Option<T> {
        self.inner
            .get(key)
            .filter(|cache_item| cache_item.valid_until > SystemTime::now())
            .map(|cache_item| cache_item.value.clone())
    }

    pub(crate) fn set(&mut self, key: &str, value: T) {
        self.inner.insert(
            key.to_string(),
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
