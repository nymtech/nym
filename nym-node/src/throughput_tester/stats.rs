// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Default)]
pub(crate) struct ClientStats {
    inner: Arc<ClientStatsInner>,
}

impl ClientStats {
    pub(crate) fn new_received(&self, new_measurement: u64) {
        const ALPHA: f64 = 0.001;
        const ONE_SUB_ALPHA: f64 = 1.0 - ALPHA;

        let old_average_latency = self.inner.average_latency_nanos.load(Ordering::SeqCst);

        let new_average = if old_average_latency == 0 {
            new_measurement
        } else {
            ((ALPHA * new_measurement as f64) + ONE_SUB_ALPHA * old_average_latency as f64) as u64
        };

        self.inner.received.fetch_add(1, Ordering::SeqCst);
        self.inner
            .average_latency_nanos
            .store(new_average, Ordering::SeqCst);
    }

    pub(crate) fn new_sent_batch(&self, batch_size: usize) {
        self.inner.sent.fetch_add(batch_size, Ordering::SeqCst);
    }

    pub(crate) fn received(&self) -> usize {
        self.inner.received.load(Ordering::SeqCst)
    }

    pub(crate) fn sent(&self) -> usize {
        self.inner.sent.load(Ordering::SeqCst)
    }

    pub(crate) fn average_latency_nanos(&self) -> u64 {
        self.inner.average_latency_nanos.load(Ordering::SeqCst)
    }

    pub(crate) fn average_latency_duration(&self) -> Duration {
        Duration::from_nanos(self.average_latency_nanos())
    }
}

#[derive(Default)]
struct ClientStatsInner {
    sent: AtomicUsize,
    received: AtomicUsize,
    average_latency_nanos: AtomicU64,
}
