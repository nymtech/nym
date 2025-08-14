// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::host::Peer;
use std::time::Duration;

const DEFAULT_PEER_MAX_FLUSHING_RATE: Duration = Duration::from_secs(60 * 60 * 24); // 24h
const DEFAULT_PEER_MAX_DELTA_FLUSHING_AMOUNT: u64 = 512 * 1024 * 1024; // 512MB

#[derive(Debug, Clone, Copy)]
pub struct PeerFlushingBehaviourConfig {
    /// Defines maximum delay between peer information being flushed to the persistent storage.
    pub peer_max_flushing_rate: Duration,

    /// Defines a maximum change in peer before it gets flushed to the persistent storage.
    pub peer_max_delta_flushing_amount: u64,
}

impl Default for PeerFlushingBehaviourConfig {
    fn default() -> Self {
        Self {
            peer_max_flushing_rate: DEFAULT_PEER_MAX_FLUSHING_RATE,
            peer_max_delta_flushing_amount: DEFAULT_PEER_MAX_DELTA_FLUSHING_AMOUNT,
        }
    }
}

pub struct CachedPeerManager {
    pub(crate) peer_information: Option<PeerInformation>,
}

impl CachedPeerManager {
    pub(crate) fn new(peer: &Peer) -> Self {
        Self {
            peer_information: Some(peer.into()),
        }
    }

    pub(crate) fn get_peer(&self) -> Option<PeerInformation> {
        self.peer_information
    }

    pub(crate) fn remove_peer(&mut self) {
        self.peer_information = None;
    }

    pub(crate) fn update(&mut self, kernel_peer: PeerInformation) {
        if let Some(peer_information) = self.peer_information.as_mut() {
            peer_information.update_tx_rx_bytes(kernel_peer);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PeerInformation {
    pub(crate) tx_bytes: u64,
    pub(crate) rx_bytes: u64,
}

impl From<&Peer> for PeerInformation {
    fn from(value: &Peer) -> Self {
        Self {
            tx_bytes: value.tx_bytes,
            rx_bytes: value.rx_bytes,
        }
    }
}

impl PeerInformation {
    pub(crate) fn update_tx_rx_bytes(&mut self, peer: PeerInformation) {
        self.tx_bytes = peer.tx_bytes;
        self.rx_bytes = peer.rx_bytes;
    }

    fn rx_tx_total(&self, typ: &'static str) -> Option<u64> {
        self.rx_bytes.checked_add(self.tx_bytes).or_else(|| {
            tracing::error!(
                "overflow on {typ} adding bytes: {} + {}",
                self.rx_bytes,
                self.tx_bytes
            );
            None
        })
    }

    /// Attempt to determine the amount of consumed bandwidth based on the current peer information
    /// and state from the last checkpoint.
    pub(crate) fn consumed_bandwidth(kernel: &Self, previous_cached: &Self) -> Option<u64> {
        let kernel_total = kernel.rx_tx_total("kernel")?;
        let cached_total = previous_cached.rx_tx_total("cached")?;
        kernel_total.checked_sub(cached_total).or_else(|| {
            tracing::error!("Overflow on spent bandwidth subtraction: kernel - cached = {kernel_total} - {cached_total}");
            None
        })
    }

    /// Attempt to determine the amount of consumed bandwidth based on the current peer information
    /// and state from the last checkpoint.
    /// On failures, it will attempt to default to most sensible alternative
    ///
    /// Note, it is responsibility of the caller to ensure that `self` corresponds to the kernel peer information
    pub(crate) fn consumed_kernel_bandwidth(&self, previous_cached: &Self) -> i64 {
        let Some(consumed) = Self::consumed_bandwidth(self, previous_cached) else {
            // old behaviour of returning the `Default::default()`
            return 0;
        };

        // old behaviour would have also returned 0 here, but I'd argue if u64 can't fit in i64,
        // it means we're over i64::MAX, thus that's what we should return
        consumed
            .try_into()
            .inspect_err(|err| tracing::error!("Could not convert from u64 to i64: {err:?}"))
            .unwrap_or(i64::MAX)
    }
}
