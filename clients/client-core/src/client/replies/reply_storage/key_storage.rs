// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use nymsphinx::anonymous_replies::encryption_key::EncryptionKeyDigest;
use nymsphinx::anonymous_replies::SurbEncryptionKey;
use std::collections::HashSet;
use std::sync::Arc;

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
