// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ReceivedReplySurbsMap, SentReplyKeys, UsedSenderTags};

#[derive(Debug, Clone)]
pub struct CombinedReplyStorage {
    sent_reply_keys: SentReplyKeys,
    received_reply_surbs: ReceivedReplySurbsMap,
    used_tags: UsedSenderTags,
}

impl CombinedReplyStorage {
    pub fn new(min_surb_threshold: usize, max_surb_threshold: usize) -> CombinedReplyStorage {
        CombinedReplyStorage {
            sent_reply_keys: SentReplyKeys::new(),
            received_reply_surbs: ReceivedReplySurbsMap::new(
                min_surb_threshold,
                max_surb_threshold,
            ),
            used_tags: UsedSenderTags::new(),
        }
    }

    pub fn load(
        sent_reply_keys: SentReplyKeys,
        received_reply_surbs: ReceivedReplySurbsMap,
        used_tags: UsedSenderTags,
    ) -> Self {
        CombinedReplyStorage {
            sent_reply_keys,
            received_reply_surbs,
            used_tags,
        }
    }

    pub fn key_storage(&self) -> SentReplyKeys {
        self.sent_reply_keys.clone()
    }

    pub fn surbs_storage(&self) -> ReceivedReplySurbsMap {
        self.received_reply_surbs.clone()
    }

    pub fn tags_storage(&self) -> UsedSenderTags {
        self.used_tags.clone()
    }

    pub fn key_storage_ref(&self) -> &SentReplyKeys {
        &self.sent_reply_keys
    }

    pub fn surbs_storage_ref(&self) -> &ReceivedReplySurbsMap {
        &self.received_reply_surbs
    }

    pub fn tags_storage_ref(&self) -> &UsedSenderTags {
        &self.used_tags
    }
}
