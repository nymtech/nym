// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use nym_credentials_interface::AvailableBandwidth;
use time::OffsetDateTime;

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

#[derive(Debug, Clone, Copy)]
pub struct ClientBandwidth {
    pub(crate) bandwidth: AvailableBandwidth,
    pub(crate) last_flushed: OffsetDateTime,

    /// the number of bytes the client had during the last sync.
    /// it is used to determine whether the current value should be synced with the storage
    /// by checking the delta with the known amount
    pub(crate) bytes_at_last_sync: i64,
    pub(crate) bytes_delta_since_sync: i64,
}

impl ClientBandwidth {
    pub fn new(bandwidth: AvailableBandwidth) -> ClientBandwidth {
        ClientBandwidth {
            bandwidth,
            last_flushed: OffsetDateTime::now_utc(),
            bytes_at_last_sync: bandwidth.bytes,
            bytes_delta_since_sync: 0,
        }
    }

    pub(crate) fn should_sync(&self, cfg: BandwidthFlushingBehaviourConfig) -> bool {
        if self.bytes_delta_since_sync.abs() >= cfg.client_bandwidth_max_delta_flushing_amount {
            return true;
        }

        if self.last_flushed + cfg.client_bandwidth_max_flushing_rate < OffsetDateTime::now_utc() {
            return true;
        }

        false
    }

    pub(crate) fn update_sync_data(&mut self) {
        self.last_flushed = OffsetDateTime::now_utc();
        self.bytes_at_last_sync = self.bandwidth.bytes;
        self.bytes_delta_since_sync = 0;
    }
}
