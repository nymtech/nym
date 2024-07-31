// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use si_scale::helpers::bibytes2;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Clone, Default)]
pub struct ClientBandwidth {
    inner: Arc<ClientBandwidthInner>,
}

#[derive(Default)]
struct ClientBandwidthInner {
    /// the actual bandwidth amount (in bytes) available
    available: AtomicI64,

    /// defines the timestamp when the bandwidth information has been logged to the logs stream
    last_logged_ts: AtomicI64,

    /// defines the timestamp when the bandwidth value was last updated
    last_updated_ts: AtomicI64,
}

impl ClientBandwidth {
    pub(crate) fn new_empty() -> Self {
        ClientBandwidth {
            inner: Arc::new(ClientBandwidthInner {
                available: AtomicI64::new(0),
                last_logged_ts: AtomicI64::new(0),
                last_updated_ts: AtomicI64::new(0),
            }),
        }
    }
    pub(crate) fn remaining(&self) -> i64 {
        self.inner.available.load(Ordering::Acquire)
    }

    pub(crate) fn maybe_log_bandwidth(&self, now: Option<OffsetDateTime>) {
        let last = self.last_logged();
        let now = now.unwrap_or_else(OffsetDateTime::now_utc);
        if last + Duration::from_secs(10) < now {
            self.log_bandwidth(Some(now))
        }
    }

    pub(crate) fn log_bandwidth(&self, now: Option<OffsetDateTime>) {
        let now = now.unwrap_or_else(OffsetDateTime::now_utc);

        let remaining = self.remaining();
        let remaining_bi2 = bibytes2(remaining as f64);

        if remaining < 0 {
            log::warn!("OUT OF BANDWIDTH. remaining: {remaining_bi2}");
        } else {
            log::info!("remaining bandwidth: {remaining_bi2}");
        }

        self.inner
            .last_logged_ts
            .store(now.unix_timestamp(), Ordering::Relaxed)
    }

    pub(crate) fn update_and_maybe_log(&self, remaining: i64) {
        let now = OffsetDateTime::now_utc();
        self.inner.available.store(remaining, Ordering::Release);
        self.inner
            .last_updated_ts
            .store(now.unix_timestamp(), Ordering::Relaxed);
        self.maybe_log_bandwidth(Some(now))
    }

    pub(crate) fn update_and_log(&self, remaining: i64) {
        let now = OffsetDateTime::now_utc();
        self.inner.available.store(remaining, Ordering::Release);
        self.inner
            .last_updated_ts
            .store(now.unix_timestamp(), Ordering::Relaxed);
        self.log_bandwidth(Some(now))
    }

    fn last_logged(&self) -> OffsetDateTime {
        // SAFETY: this value is always populated with valid timestamps
        OffsetDateTime::from_unix_timestamp(self.inner.last_logged_ts.load(Ordering::Relaxed))
            .unwrap()
    }
}
