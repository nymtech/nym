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
            peer_information.update_trx_bytes(kernel_peer);
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
    pub(crate) fn update_trx_bytes(&mut self, peer: PeerInformation) {
        self.tx_bytes = peer.tx_bytes;
        self.rx_bytes = peer.rx_bytes;
    }
}
