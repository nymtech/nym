// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::ecash::utils::ecash_today;
use nym_credentials_interface::AvailableBandwidth;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};

const DEFAULT_CLIENT_BANDWIDTH_MAX_FLUSHING_RATE: Duration = Duration::from_secs(5 * 60); // 5 minutes
const DEFAULT_CLIENT_BANDWIDTH_MAX_DELTA_FLUSHING_AMOUNT: i64 = 5 * 1024 * 1024; // 5MB

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
    sync_lock: Arc<Mutex<()>>,
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
            sync_lock: Arc::new(Mutex::new(())),
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

    pub async fn available(&self) -> i64 {
        self.inner.read().await.bandwidth.bytes
    }

    #[cfg(test)]
    pub(crate) async fn delta_since_sync(&self) -> i64 {
        self.inner.read().await.bytes_delta_since_sync
    }

    pub(crate) async fn sync_guard(&self) -> OwnedMutexGuard<()> {
        self.sync_lock.clone().lock_owned().await
    }

    pub(crate) async fn take_delta_since_sync(&self) -> i64 {
        let mut guard = self.inner.write().await;
        let delta = guard.bytes_delta_since_sync;
        guard.bytes_delta_since_sync = 0;
        delta
    }

    pub(crate) async fn restore_delta_since_sync(&self, delta: i64) {
        let mut guard = self.inner.write().await;
        guard.bytes_delta_since_sync += delta;
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

        guard.bytes_at_last_sync = stored;
        guard.bandwidth.bytes = stored + guard.bytes_delta_since_sync;
        guard.last_synced = OffsetDateTime::now_utc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bandwidth(bytes: i64) -> ClientBandwidth {
        ClientBandwidth::new(AvailableBandwidth {
            bytes,
            expiration: OffsetDateTime::UNIX_EPOCH,
        })
    }

    #[tokio::test]
    async fn resync_preserves_delta_accumulated_during_storage_sync() {
        let bandwidth = test_bandwidth(1_000);

        bandwidth.decrease_bandwidth(100).await;
        let reserved_delta = bandwidth.take_delta_since_sync().await;
        assert_eq!(reserved_delta, -100);
        assert_eq!(bandwidth.delta_since_sync().await, 0);

        bandwidth.decrease_bandwidth(50).await;
        bandwidth.resync_bandwidth_with_storage(900).await;

        assert_eq!(bandwidth.available().await, 850);
        assert_eq!(bandwidth.delta_since_sync().await, -50);
    }

    #[tokio::test]
    async fn failed_sync_restores_reserved_delta() {
        let bandwidth = test_bandwidth(1_000);

        bandwidth.decrease_bandwidth(100).await;
        let reserved_delta = bandwidth.take_delta_since_sync().await;
        bandwidth.decrease_bandwidth(25).await;
        bandwidth.restore_delta_since_sync(reserved_delta).await;

        assert_eq!(bandwidth.available().await, 875);
        assert_eq!(bandwidth.delta_since_sync().await, -125);
    }

    #[tokio::test]
    async fn old_read_only_sync_could_apply_the_same_delta_twice() {
        let old_behaviour = test_bandwidth(1_000);
        old_behaviour.decrease_bandwidth(100).await;

        let old_first_sync_delta = old_behaviour.delta_since_sync().await;
        let old_second_sync_delta = old_behaviour.delta_since_sync().await;
        let old_stored = 1_000 + old_first_sync_delta + old_second_sync_delta;

        assert_eq!(old_first_sync_delta, -100);
        assert_eq!(old_second_sync_delta, -100);
        assert_eq!(old_stored, 800);

        let new_behaviour = test_bandwidth(1_000);
        new_behaviour.decrease_bandwidth(100).await;

        let new_first_sync_delta = new_behaviour.take_delta_since_sync().await;
        let new_second_sync_delta = new_behaviour.take_delta_since_sync().await;
        let new_stored = 1_000 + new_first_sync_delta + new_second_sync_delta;

        assert_eq!(new_first_sync_delta, -100);
        assert_eq!(new_second_sync_delta, 0);
        assert_eq!(new_stored, 900);
    }
}
