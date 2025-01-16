// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct WireguardStats {
    bytes_rx: AtomicUsize,
    bytes_tx: AtomicUsize,

    total_peers: AtomicUsize,
    active_peers: AtomicUsize,
}

impl WireguardStats {
    pub fn bytes_rx(&self) -> usize {
        self.bytes_rx.load(Ordering::Relaxed)
    }

    pub fn bytes_tx(&self) -> usize {
        self.bytes_tx.load(Ordering::Relaxed)
    }

    pub fn total_peers(&self) -> usize {
        self.total_peers.load(Ordering::Relaxed)
    }

    pub fn active_peers(&self) -> usize {
        self.active_peers.load(Ordering::Relaxed)
    }

    pub fn update(
        &self,
        new_bytes_rx: usize,
        new_bytes_tx: usize,
        total_peers: usize,
        active_peers: usize,
    ) {
        self.bytes_rx.fetch_add(new_bytes_rx, Ordering::Relaxed);
        self.bytes_tx.fetch_add(new_bytes_tx, Ordering::Relaxed);
        self.total_peers.store(total_peers, Ordering::Relaxed);
        self.active_peers.store(active_peers, Ordering::Relaxed);
    }
}
