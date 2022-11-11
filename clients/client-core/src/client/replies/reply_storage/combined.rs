// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::replies::reply_storage::{ReceivedReplySurbsMap, SentReplyKeys};
use nymsphinx::anonymous_replies::encryption_key::EncryptionKeyDigest;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::{ReplySurb, SurbEncryptionKey};

#[derive(Debug, Clone)]
pub struct CombinedReplyStorage {
    sent_reply_keys: SentReplyKeys,
    received_reply_surbs: ReceivedReplySurbsMap,
}

impl CombinedReplyStorage {
    pub fn new(min_surb_threshold: usize) -> CombinedReplyStorage {
        CombinedReplyStorage {
            sent_reply_keys: SentReplyKeys::new(),
            received_reply_surbs: ReceivedReplySurbsMap::new(min_surb_threshold),
        }
    }

    #[deprecated]
    pub fn key_storage(&self) -> SentReplyKeys {
        self.sent_reply_keys.clone()
    }

    #[deprecated]
    pub(crate) fn surbs_storage(&self) -> ReceivedReplySurbsMap {
        self.received_reply_surbs.clone()
    }

    pub(crate) fn min_surb_threshold(&self) -> usize {
        self.received_reply_surbs.min_surb_threshold()
    }

    pub(crate) fn available_surbs(&self, target: &AnonymousSenderTag) -> usize {
        self.received_reply_surbs.available_surbs(target)
    }

    pub(crate) fn contains_surbs_for(&self, target: &AnonymousSenderTag) -> bool {
        self.received_reply_surbs.contains_surbs_for(target)
    }

    pub(crate) fn get_reply_surbs(
        &self,
        target: &AnonymousSenderTag,
        amount: usize,
    ) -> (Option<Vec<ReplySurb>>, usize) {
        self.received_reply_surbs.get_reply_surbs(target, amount)
    }

    pub(crate) fn get_reply_surb_ignoring_threshold(
        &self,
        target: &AnonymousSenderTag,
    ) -> Option<(Option<ReplySurb>, usize)> {
        self.received_reply_surbs
            .get_reply_surb_ignoring_threshold(target)
    }

    pub(crate) fn insert_surbs<I: IntoIterator<Item = ReplySurb>>(
        &self,
        target: &AnonymousSenderTag,
        surbs: I,
    ) {
        self.received_reply_surbs.insert_surbs(target, surbs)
    }

    pub(crate) fn insert_multiple_surb_keys(&self, keys: Vec<SurbEncryptionKey>) {
        self.sent_reply_keys.insert_multiple(keys)
    }

    pub(crate) fn insert_surb_key(&self, key: SurbEncryptionKey) {
        self.sent_reply_keys.insert(key)
    }

    pub(crate) fn try_pop_surb_key(
        &self,
        digest: EncryptionKeyDigest,
    ) -> Option<SurbEncryptionKey> {
        self.sent_reply_keys.try_pop(digest)
    }
}
