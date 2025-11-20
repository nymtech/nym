// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::support::caching::refresher::RefreshRequester;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub(crate) struct Refreshing {
    handle: RefreshRequester,
    last_requested: Arc<AtomicI64>, // unix timestamp
}

impl Refreshing {
    pub(crate) fn new(handle: RefreshRequester) -> Self {
        Refreshing {
            handle,
            last_requested: Arc::new(Default::default()),
        }
    }

    pub(crate) fn last_requested(&self) -> OffsetDateTime {
        // SAFETY: this value is always populated with valid timestamps
        #[allow(clippy::unwrap_used)]
        OffsetDateTime::from_unix_timestamp(self.last_requested.load(Ordering::SeqCst)).unwrap()
    }

    fn update_last_requested(&self, now: OffsetDateTime) {
        self.last_requested
            .store(now.unix_timestamp(), Ordering::SeqCst);
    }

    pub(crate) fn request_refresh(&self, now: OffsetDateTime) {
        self.update_last_requested(now);
        self.handle.request_cache_refresh();
    }
}
