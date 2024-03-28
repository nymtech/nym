use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, SystemTime};

const DEFAULT_CACHE_VALIDITY: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub(crate) struct Cache<K, V: Clone> {
    inner: HashMap<K, CacheItem<V>>,
    cache_validity_duration: Duration,
}

impl<K, V: Clone> Cache<K, V>
where
    K: Eq + Hash,
{
    pub(crate) fn new() -> Self {
        Cache {
            inner: HashMap::new(),
            cache_validity_duration: DEFAULT_CACHE_VALIDITY,
        }
    }

    // it felt like this might be an useful addition if we want to keep our caches with different
    // validity durations
    #[allow(unused)]
    pub(crate) fn with_validity_duration(mut self, new_cache_validity: Duration) -> Self {
        self.cache_validity_duration = new_cache_validity;
        self
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn get_all(&self) -> Vec<V> {
        self.inner
            .values()
            .map(|cache_item| cache_item.value.clone())
            .collect()
    }

    pub(crate) fn get<Q>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.inner
            .get(key)
            .filter(|cache_item| cache_item.valid_until >= SystemTime::now())
            .map(|cache_item| cache_item.value.clone())
    }

    pub(crate) fn set(&mut self, key: K, value: V) {
        self.inner.insert(
            key,
            CacheItem {
                valid_until: SystemTime::now() + self.cache_validity_duration,
                value,
            },
        );
    }

    #[allow(unused)]
    pub(crate) fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.inner.remove(key).map(|item| item.value)
    }

    #[allow(unused)]
    pub(crate) fn remove_if_expired<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        if self.inner.get(key)?.has_expired() {
            self.remove(key)
        } else {
            None
        }
    }

    // it seems like something should be running on timer calling this method on all of our caches
    #[allow(unused)]
    pub(crate) fn remove_all_expired(&mut self) {
        self.inner.retain(|_, v| !v.has_expired())
    }
}

#[derive(Clone)]
pub(crate) struct CacheItem<T> {
    pub(crate) value: T,
    pub(crate) valid_until: std::time::SystemTime,
}

impl<T> CacheItem<T> {
    fn has_expired(&self) -> bool {
        let now = SystemTime::now();
        self.valid_until < now
    }
}
