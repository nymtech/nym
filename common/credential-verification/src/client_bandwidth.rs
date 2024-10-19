// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::ecash::utils::ecash_today;
use nym_credentials_interface::AvailableBandwidth;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;

const DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE: Duration = Duration::from_millis(5);
const DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT: i64 = 512 * 1024; // 512kB

#[derive(Debug, Clone, Copy)]
pub struct BandwidthFlushingBehaviourConfig {
    /// Defines maximum delay between client bandwidth information being flushed to the persistent storage.
    pub client_bandwidth_max_flushing_rate: Duration,

    /// Defines a maximum change in client bandwidth before it gets flushed to the persistent storage.
    pub client_bandwidth_max_delta_flushing_amount: i64,
}

impl Default for BandwidthFlushingBehaviourConfig {
    fn default() -> Self {
        Self {
            client_bandwidth_max_flushing_rate: DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE,
            client_bandwidth_max_delta_flushing_amount:
                DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientBandwidth {
    inner: Arc<RwLock<ClientBandwidthInner>>,
}

#[derive(Debug)]
struct ClientBandwidthInner {
    pub(crate) bandwidth: AvailableBandwidth,
    pub(crate) last_synced: OffsetDateTime,

    /// the number of bytes the client had during the last sync.
    /// it is used to determine whether the current value should be synced with the storage
    /// by checking the delta with the known amount
    pub(crate) bytes_at_last_sync: i64,
    pub(crate) bytes_delta_since_sync: i64,
}

impl ClientBandwidth {
    pub fn new(bandwidth: AvailableBandwidth) -> ClientBandwidth {
        ClientBandwidth {
            inner: Arc::new(RwLock::new(ClientBandwidthInner {
                bandwidth,
                last_synced: OffsetDateTime::now_utc(),
                bytes_at_last_sync: bandwidth.bytes,
                bytes_delta_since_sync: 0,
            })),
        }
    }

    pub(crate) async fn should_sync(&self, cfg: BandwidthFlushingBehaviourConfig) -> bool {
        let guard = self.inner.read().await;

        if guard.bytes_delta_since_sync.abs() >= cfg.client_bandwidth_max_delta_flushing_amount {
            return true;
        }

        if guard.last_synced + cfg.client_bandwidth_max_flushing_rate < OffsetDateTime::now_utc() {
            return true;
        }

        false
    }

    pub(crate) async fn available(&self) -> i64 {
        self.inner.read().await.bandwidth.bytes
    }

    pub(crate) async fn delta_since_sync(&self) -> i64 {
        self.inner.read().await.bytes_delta_since_sync
    }
    pub(crate) async fn expiration(&self) -> OffsetDateTime {
        self.inner.read().await.bandwidth.expiration
    }

    pub(crate) async fn expired(&self) -> bool {
        self.expiration().await < ecash_today()
    }

    pub(crate) async fn decrease_bandwidth(&self, decrease: i64) {
        let mut guard = self.inner.write().await;

        guard.bandwidth.bytes -= decrease;
        guard.bytes_delta_since_sync -= decrease;
    }

    pub(crate) async fn increase_bandwidth(&self, increase: i64, new_expiration: OffsetDateTime) {
        let mut guard = self.inner.write().await;

        guard.bandwidth.bytes += increase;
        guard.bandwidth.expiration = new_expiration;
        guard.bytes_delta_since_sync += increase;
    }

    pub(crate) async fn expire_bandwidth(&self) {
        let mut guard = self.inner.write().await;

        guard.bandwidth = AvailableBandwidth::default();
        guard.last_synced = OffsetDateTime::now_utc();
        guard.bytes_at_last_sync = 0;
        guard.bytes_delta_since_sync = 0;
    }

    pub(crate) async fn resync_bandwidth_with_storage(&self, stored: i64) {
        let mut guard = self.inner.write().await;

        guard.bandwidth.bytes = stored;
        guard.bytes_at_last_sync = stored;
        guard.bytes_delta_since_sync = 0;
        guard.last_synced = OffsetDateTime::now_utc();
    }
}
