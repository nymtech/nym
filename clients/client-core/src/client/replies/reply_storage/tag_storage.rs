// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use nymsphinx::addressing::clients::{Recipient, RecipientBytes};
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct UsedSenderTags {
    inner: Arc<UsedSenderTagsInner>,
}

#[derive(Debug)]
struct UsedSenderTagsInner {
    data: DashMap<RecipientBytes, AnonymousSenderTag>,
}

impl UsedSenderTags {
    pub(crate) fn new() -> UsedSenderTags {
        UsedSenderTags {
            inner: Arc::new(UsedSenderTagsInner {
                data: DashMap::new(),
            }),
        }
    }

    pub(crate) fn insert_new(&self, recipient: &Recipient, tag: AnonymousSenderTag) {
        self.inner.data.insert(recipient.to_bytes(), tag);
    }

    pub(crate) fn try_get_existing(&self, recipient: &Recipient) -> Option<AnonymousSenderTag> {
        self.inner
            .data
            .get(&recipient.to_bytes())
            .map(|r| *r.value())
    }
}
