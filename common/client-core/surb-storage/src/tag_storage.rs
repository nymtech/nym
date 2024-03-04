// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use nym_sphinx::addressing::clients::{Recipient, RecipientBytes};
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use std::sync::Arc;

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
use dashmap::iter::Iter;

#[derive(Debug, Clone)]
pub struct UsedSenderTags {
    inner: Arc<UsedSenderTagsInner>,
}

impl Default for UsedSenderTags {
    fn default() -> Self {
        UsedSenderTags::new()
    }
}

#[derive(Debug)]
struct UsedSenderTagsInner {
    data: DashMap<RecipientBytes, AnonymousSenderTag>,
}

impl UsedSenderTags {
    pub fn new() -> UsedSenderTags {
        UsedSenderTags {
            inner: Arc::new(UsedSenderTagsInner {
                data: DashMap::new(),
            }),
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn from_raw(raw: Vec<(RecipientBytes, AnonymousSenderTag)>) -> UsedSenderTags {
        UsedSenderTags {
            inner: Arc::new(UsedSenderTagsInner {
                data: raw.into_iter().collect(),
            }),
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn as_raw_iter(&self) -> Iter<'_, RecipientBytes, AnonymousSenderTag> {
        self.inner.data.iter()
    }

    pub fn insert_new(&self, recipient: &Recipient, tag: AnonymousSenderTag) {
        self.inner.data.insert(recipient.to_bytes(), tag);
    }

    pub fn try_get_existing(&self, recipient: &Recipient) -> Option<AnonymousSenderTag> {
        self.inner
            .data
            .get(&recipient.to_bytes())
            .map(|r| *r.value())
    }

    pub fn exists(&self, recipient: &Recipient) -> bool {
        self.inner.data.contains_key(&recipient.to_bytes())
    }
}
