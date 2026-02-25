// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use bincode::Options;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockMappedWriteGuard, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, error};

#[derive(Debug, Error)]
#[error("the cache item has not been initialised")]
pub struct UninitialisedCache;

pub struct SharedCache<T>(Arc<RwLock<CachedItem<T>>>);

impl<T> Clone for SharedCache<T> {
    fn clone(&self) -> Self {
        SharedCache(Arc::clone(&self.0))
    }
}

impl<T> Default for SharedCache<T> {
    fn default() -> Self {
        SharedCache(Arc::new(RwLock::new(CachedItem { inner: None })))
    }
}

impl<T> SharedCache<T> {
    pub(crate) fn new() -> Self {
        SharedCache::default()
    }

    #[track_caller]
    pub(crate) fn new_with_persistent<P: AsRef<Path>>(
        store_path: P,
        max_cache_age: Duration,
        fallback_value: Option<T>,
    ) -> Self
    where
        T: DeserializeOwned,
    {
        // attempt to load data from disk
        let Ok(disk_cached) = Cache::<T>::try_deserialise_from_file(store_path) else {
            // if failed, fallback to fresh state
            // (the file might not have existed, for example on initial run)
            return if let Some(fallback_value) = fallback_value {
                Self::new_with_value(fallback_value)
            } else {
                Self::new()
            };
        };
        // check if the entry is not too stale
        if disk_cached.has_expired(max_cache_age, None) {
            // if too old, fallback to fresh state
            debug!("cache has expired");
            return if let Some(fallback_value) = fallback_value {
                Self::new_with_value(fallback_value)
            } else {
                Self::new()
            };
        }
        // use loaded value
        SharedCache(Arc::new(RwLock::new(CachedItem {
            inner: Some(disk_cached),
        })))
    }

    pub(crate) fn new_with_value(value: T) -> Self {
        SharedCache(Arc::new(RwLock::new(CachedItem {
            inner: Some(Cache::new(value)),
        })))
    }

    pub(crate) async fn try_update_value<S>(
        &self,
        update: S,
        update_fn: impl Fn(&mut T, S),
        typ: &str,
    ) -> Result<(), S>
    where
        S: Into<T>,
    {
        let update_value = update;
        let mut guard = match tokio::time::timeout(Duration::from_millis(200), self.0.write()).await
        {
            Ok(guard) => guard,
            Err(_) => {
                debug!("failed to obtain write permit for {typ} cache");
                return Err(update_value);
            }
        };

        if let Some(ref mut existing) = guard.inner {
            existing.update(update_value, update_fn);
        } else {
            guard.inner = Some(Cache::new(update_value.into()))
        };
        Ok(())
    }

    pub(crate) async fn try_overwrite_old_value(
        &self,
        value: impl Into<T>,
        typ: &str,
    ) -> Result<(), T> {
        let value = value.into();
        let mut guard = match tokio::time::timeout(Duration::from_millis(200), self.0.write()).await
        {
            Ok(guard) => guard,
            Err(_) => {
                debug!("failed to obtain write permit for {typ} cache");
                return Err(value);
            }
        };

        if let Some(ref mut existing) = guard.inner {
            existing.unchecked_update(value)
        } else {
            guard.inner = Some(Cache::new(value))
        };
        Ok(())
    }

    pub(crate) async fn get(&self) -> Result<RwLockReadGuard<'_, Cache<T>>, UninitialisedCache> {
        let guard = self.0.read().await;
        RwLockReadGuard::try_map(guard, |a| a.inner.as_ref()).map_err(|_| UninitialisedCache)
    }

    pub(crate) async fn write(
        &self,
    ) -> Result<RwLockMappedWriteGuard<'_, Cache<T>>, UninitialisedCache> {
        let guard = self.0.write().await;
        RwLockWriteGuard::try_map(guard, |a| a.inner.as_mut()).map_err(|_| UninitialisedCache)
    }

    // ignores expiration data
    #[allow(dead_code)]
    pub(crate) async fn unchecked_get_inner(
        &self,
    ) -> Result<RwLockReadGuard<'_, T>, UninitialisedCache> {
        Ok(RwLockReadGuard::map(self.get().await?, |a| &a.value))
    }

    pub(crate) async fn naive_wait_for_initial_values(&self) {
        let initialisation_backoff = Duration::from_secs(5);
        loop {
            if self.get().await.is_ok() {
                break;
            } else {
                tokio::time::sleep(initialisation_backoff).await;
            }
        }
    }
}

impl<T> From<Cache<T>> for SharedCache<T> {
    fn from(value: Cache<T>) -> Self {
        SharedCache(Arc::new(RwLock::new(value.into())))
    }
}

#[derive(Default)]
pub(crate) struct CachedItem<T> {
    inner: Option<Cache<T>>,
}

impl<T> CachedItem<T> {
    #[allow(dead_code)]
    fn get(&self) -> Result<&Cache<T>, UninitialisedCache> {
        self.inner.as_ref().ok_or(UninitialisedCache)
    }
}

impl<T> From<Cache<T>> for CachedItem<T> {
    fn from(value: Cache<T>) -> Self {
        CachedItem { inner: Some(value) }
    }
}

// specialised variant of `Cache` for holding maps of values that allow updates to individual entries

/*
   pub(crate) fn partial_update<F>(&mut self, partial_value: impl Into<S>, update_fn: F)
   where
       F: FnOnce(&mut T, S),
   {
       update_fn(&mut self.value, partial_value.into());
       self.as_at = OffsetDateTime::now_utc()
   }

*/

// don't use this directly!
// opt for SharedCache<T> instead
pub struct Cache<T> {
    value: T,
    as_at: OffsetDateTime,
}

impl<T> Cache<Option<T>> {
    #[allow(dead_code)]
    pub(crate) fn transpose(self) -> Option<Cache<T>> {
        self.value.map(|value| Cache {
            value,
            as_at: self.as_at,
        })
    }
}

impl<T> Cache<T> {
    fn new(value: T) -> Self {
        Cache {
            value,
            as_at: OffsetDateTime::now_utc(),
        }
    }

    // I know, it's dead code for now, but I feel it could be useful code in the future
    #[allow(dead_code)]
    pub(crate) fn map<F, U>(this: Self, f: F) -> Cache<U>
    where
        F: FnOnce(T) -> U,
    {
        Cache {
            value: f(this.value),
            as_at: this.as_at,
        }
    }

    pub(crate) fn as_mapped<F, U>(this: &Self, f: F) -> Cache<U>
    where
        F: Fn(&T) -> U,
    {
        Cache {
            value: f(&this.value),
            as_at: this.as_at,
        }
    }

    pub(crate) fn update<S>(&mut self, update: S, update_fn: impl Fn(&mut T, S)) {
        update_fn(&mut self.value, update);
        self.as_at = OffsetDateTime::now_utc();
    }

    // ugh. I hate to expose it, but it'd have broken pre-existing code
    pub(crate) fn unchecked_update(&mut self, value: impl Into<T>) {
        self.value = value.into();
        self.as_at = OffsetDateTime::now_utc()
    }

    pub(crate) fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    #[allow(dead_code)]
    pub fn has_expired(&self, ttl: Duration, now: Option<OffsetDateTime>) -> bool {
        let now = now.unwrap_or(OffsetDateTime::now_utc());
        let diff = now - self.as_at;

        diff > ttl
    }

    pub fn timestamp(&self) -> OffsetDateTime {
        self.as_at
    }

    #[track_caller]
    pub(crate) fn try_serialise_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()>
    where
        T: Serialize,
    {
        SerialisableCache {
            value: &self.value,
            as_at: self.as_at,
        }
        .try_serialise_to_file(path)
    }

    #[track_caller]
    pub(crate) fn try_deserialise_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self>
    where
        T: DeserializeOwned,
    {
        let path = path.as_ref();
        if !path.exists() {
            debug!("cached file does not exist at: {}", path.display());
            return Err(std::io::Error::other("cached file does not exist"));
        }

        DeserialisedCache::try_deserialise_from_file(path).map(|d| Cache {
            value: d.value,
            as_at: d.as_at,
        })
    }
}

impl<T> Deref for Cache<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Default for Cache<T>
where
    T: Default,
{
    fn default() -> Self {
        Cache {
            value: T::default(),
            as_at: OffsetDateTime::UNIX_EPOCH,
        }
    }
}

#[derive(Serialize)]
struct SerialisableCache<'a, T> {
    value: &'a T,

    #[serde(with = "time::serde::rfc3339")]
    as_at: OffsetDateTime,
}

impl<'a, T> SerialisableCache<'a, T> {
    #[track_caller]
    fn try_serialise_to_file<P: AsRef<Path>>(self, path: P) -> std::io::Result<()>
    where
        T: Serialize,
    {
        use ::bincode::Options;

        let serialiser = make_bincode_serializer();
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = match File::create(path) {
            Ok(file) => file,
            Err(err) => {
                error!("failed to create persistent cache file at {path:?}: {err}",);
                return Err(err);
            }
        };

        serialiser.serialize_into(file, &self).map_err(|err| {
            error!("failed to serialise persistent cache file at {path:?}: {err}");
            std::io::Error::other(err)
        })
    }
}

#[derive(Deserialize)]
struct DeserialisedCache<T> {
    value: T,

    #[serde(with = "time::serde::rfc3339")]
    as_at: OffsetDateTime,
}

impl<T> DeserialisedCache<T> {
    #[track_caller]
    fn try_deserialise_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self>
    where
        T: DeserializeOwned,
    {
        use ::bincode::Options;

        let serialiser = make_bincode_serializer();
        let path = path.as_ref();

        let file = match File::open(path) {
            Ok(file) => file,
            Err(err) => {
                error!("failed to open persistent cache file at {path:?}: {err}",);
                return Err(err);
            }
        };

        serialiser.deserialize_from(file).map_err(|err| {
            error!("failed to deserialised persistent cache file at {path:?}: {err}");
            std::io::Error::other(err)
        })
    }
}

fn make_bincode_serializer() -> impl ::bincode::Options {
    ::bincode::DefaultOptions::new()
        .with_little_endian()
        .with_varint_encoding()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialisation_is_reciprocal() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let dummy_data = Cache {
            value: "foomp".to_string(),
            as_at: OffsetDateTime::now_utc(),
        };

        dummy_data.try_serialise_to_file(tmp.path()).unwrap();
        let de = Cache::<String>::try_deserialise_from_file(tmp.path()).unwrap();
        assert_eq!(dummy_data.value, de.value);
        assert_eq!(dummy_data.as_at, de.as_at);
    }
}
