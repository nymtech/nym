// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use nymsphinx::anonymous_replies::encryption_key::EncryptionKeyDigest;
use nymsphinx::anonymous_replies::SurbEncryptionKey;
use std::sync::Arc;

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
use dashmap::iter::Iter;

#[derive(Debug, Clone)]
// TODO: we might have to also put the tag here
// TODO2: some timestamp to indicate when entries should get purged if we expect to never get the reply back
pub struct SentReplyKeys {
    inner: Arc<SentReplyKeysInner>,
}

#[derive(Debug)]
struct SentReplyKeysInner {
    data: DashMap<EncryptionKeyDigest, SurbEncryptionKey>,
}

impl SentReplyKeys {
    pub(crate) fn new() -> SentReplyKeys {
        SentReplyKeys {
            inner: Arc::new(SentReplyKeysInner {
                data: DashMap::new(),
            }),
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub(crate) fn from_raw(raw: Vec<(EncryptionKeyDigest, SurbEncryptionKey)>) -> SentReplyKeys {
        SentReplyKeys {
            inner: Arc::new(SentReplyKeysInner {
                data: raw.into_iter().collect(),
            }),
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub(crate) fn as_raw_iter(&self) -> Iter<'_, EncryptionKeyDigest, SurbEncryptionKey> {
        self.inner.data.iter()
    }

    pub(crate) fn insert_multiple(&self, keys: Vec<SurbEncryptionKey>) {
        for key in keys {
            self.insert(key)
        }
    }

    pub(crate) fn insert(&self, key: SurbEncryptionKey) {
        self.inner.data.insert(key.compute_digest(), key);
    }

    pub(crate) fn try_pop(&self, digest: EncryptionKeyDigest) -> Option<SurbEncryptionKey> {
        self.inner.data.remove(&digest).map(|(_k, v)| v)
    }
}
