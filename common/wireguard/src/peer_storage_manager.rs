// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::host::Peer;
use std::time::Duration;
use time::OffsetDateTime;

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
    pub(crate) cfg: PeerFlushingBehaviourConfig,
}

impl CachedPeerManager {
    pub(crate) fn new(peer: Peer) -> Self {
        let peer_information = Some(PeerInformation::new(peer));
        Self {
            peer_information,
            cfg: PeerFlushingBehaviourConfig::default(),
        }
    }

    pub(crate) fn get_peer(&self) -> Option<Peer> {
        self.peer_information.as_ref().map(|p| p.peer.clone())
    }

    pub(crate) fn remove_peer(&mut self) {
        self.peer_information = None;
    }

    pub(crate) fn update_trx(&mut self, kernel_peer: &Peer) {
        if let Some(peer_information) = self.peer_information.as_mut() {
            peer_information.update_trx_bytes(kernel_peer.tx_bytes, kernel_peer.rx_bytes);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PeerInformation {
    pub(crate) peer: Peer,
    pub(crate) last_synced: OffsetDateTime,

    pub(crate) bytes_delta_since_sync: u64,
    pub(crate) force_sync: bool,
}

impl PeerInformation {
    pub fn new(peer: Peer) -> PeerInformation {
        PeerInformation {
            peer,
            last_synced: OffsetDateTime::now_utc(),
            bytes_delta_since_sync: 0,
            force_sync: false,
        }
    }

    pub(crate) fn should_sync(&self, cfg: PeerFlushingBehaviourConfig) -> bool {
        if self.force_sync {
            return true;
        }
        if self.bytes_delta_since_sync >= cfg.peer_max_delta_flushing_amount {
            return true;
        }

        if self.last_synced + cfg.peer_max_flushing_rate < OffsetDateTime::now_utc()
            && self.bytes_delta_since_sync != 0
        {
            return true;
        }

        false
    }

    pub(crate) fn peer(&self) -> &Peer {
        &self.peer
    }

    pub(crate) fn update_trx_bytes(&mut self, tx_bytes: u64, rx_bytes: u64) {
        self.bytes_delta_since_sync += tx_bytes.saturating_sub(self.peer.tx_bytes)
            + rx_bytes.saturating_sub(self.peer.rx_bytes);
        self.peer.tx_bytes = tx_bytes;
        self.peer.rx_bytes = rx_bytes;
    }

    pub(crate) fn resync_peer_with_storage(&mut self) {
        self.bytes_delta_since_sync = 0;
        self.last_synced = OffsetDateTime::now_utc();
        self.force_sync = false;
    }
}
