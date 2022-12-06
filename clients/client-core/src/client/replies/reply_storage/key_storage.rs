// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::iter::Iter;
use dashmap::DashMap;
use nymsphinx::anonymous_replies::encryption_key::EncryptionKeyDigest;
use nymsphinx::anonymous_replies::SurbEncryptionKey;
use std::ops::Deref;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::Instant;

#[cfg(target_arch = "wasm32")]
use wasm_timer::Instant;

#[derive(Debug, Clone)]
pub struct SentReplyKeys {
    inner: Arc<SentReplyKeysInner>,
}

#[derive(Debug)]
struct SentReplyKeysInner {
    data: DashMap<EncryptionKeyDigest, UsedReplyKey>,
}

impl SentReplyKeys {
    pub(crate) fn new() -> SentReplyKeys {
        SentReplyKeys {
            inner: Arc::new(SentReplyKeysInner {
                data: DashMap::new(),
            }),
        }
    }

    pub(crate) fn from_raw(raw: Vec<(EncryptionKeyDigest, UsedReplyKey)>) -> SentReplyKeys {
        SentReplyKeys {
            inner: Arc::new(SentReplyKeysInner {
                data: raw.into_iter().collect(),
            }),
        }
    }

    pub(crate) fn as_raw_iter(&self) -> Iter<'_, EncryptionKeyDigest, UsedReplyKey> {
        self.inner.data.iter()
    }

    pub(crate) fn insert_multiple(&self, keys: Vec<SurbEncryptionKey>) {
        let now = Instant::now();
        for key in keys {
            self.insert(UsedReplyKey::new(key, now))
        }
    }

    pub(crate) fn insert(&self, key: UsedReplyKey) {
        self.inner.data.insert(key.compute_digest(), key);
    }

    pub(crate) fn try_pop(&self, digest: EncryptionKeyDigest) -> Option<UsedReplyKey> {
        self.inner.data.remove(&digest).map(|(_k, v)| v)
    }

    pub(crate) fn remove(&self, digest: EncryptionKeyDigest) {
        self.inner.data.remove(&digest);
    }
}

#[derive(Debug)]
pub(crate) struct UsedReplyKey {
    key: SurbEncryptionKey,
    pub(crate) sent_at: Instant,
}

impl UsedReplyKey {
    fn new(key: SurbEncryptionKey, sent_at: Instant) -> Self {
        UsedReplyKey { key, sent_at }
    }
}

impl Deref for UsedReplyKey {
    type Target = SurbEncryptionKey;

    fn deref(&self) -> &Self::Target {
        &self.key
    }
}
