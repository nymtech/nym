// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::iter::Iter;
use dashmap::DashMap;
use nym_sphinx::anonymous_replies::encryption_key::EncryptionKeyDigest;
use nym_sphinx::anonymous_replies::SurbEncryptionKey;
use std::ops::Deref;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct SentReplyKeys {
    inner: Arc<SentReplyKeysInner>,
}

impl Default for SentReplyKeys {
    fn default() -> Self {
        SentReplyKeys::new()
    }
}

#[derive(Debug)]
struct SentReplyKeysInner {
    data: DashMap<EncryptionKeyDigest, UsedReplyKey>,
}

impl SentReplyKeys {
    pub fn new() -> SentReplyKeys {
        SentReplyKeys {
            inner: Arc::new(SentReplyKeysInner {
                data: DashMap::new(),
            }),
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn from_raw(raw: Vec<(EncryptionKeyDigest, UsedReplyKey)>) -> SentReplyKeys {
        SentReplyKeys {
            inner: Arc::new(SentReplyKeysInner {
                data: raw.into_iter().collect(),
            }),
        }
    }

    pub fn as_raw_iter(&self) -> Iter<'_, EncryptionKeyDigest, UsedReplyKey> {
        self.inner.data.iter()
    }

    pub fn insert_multiple(&self, keys: Vec<SurbEncryptionKey>) {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        for key in keys {
            self.insert(UsedReplyKey::new(key, now))
        }
    }

    pub fn insert(&self, key: UsedReplyKey) {
        self.inner.data.insert(key.compute_digest(), key);
    }

    pub fn try_pop(&self, digest: EncryptionKeyDigest) -> Option<UsedReplyKey> {
        self.inner.data.remove(&digest).map(|(_k, v)| v)
    }

    pub fn remove(&self, digest: EncryptionKeyDigest) {
        self.inner.data.remove(&digest);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct UsedReplyKey {
    key: SurbEncryptionKey,
    // the purpose of this field is to perform invalidation at relatively very long intervals
    pub sent_at_timestamp: i64,
}

impl UsedReplyKey {
    pub(crate) fn new(key: SurbEncryptionKey, sent_at_timestamp: i64) -> Self {
        UsedReplyKey {
            key,
            sent_at_timestamp,
        }
    }
}

impl Deref for UsedReplyKey {
    type Target = SurbEncryptionKey;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}
