// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct NetworkStats {
    // for now just experiment with basic data, we could always extend it
    active_ingress_mixnet_connections: AtomicUsize,
}

impl NetworkStats {
    pub fn new_active_ingress_mixnet_client(&self) {
        self.active_ingress_mixnet_connections
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn disconnected_ingress_mixnet_client(&self) {
        self.active_ingress_mixnet_connections
            .fetch_sub(1, Ordering::Relaxed);
    }

    pub fn active_ingress_mixnet_connections_count(&self) -> usize {
        self.active_ingress_mixnet_connections
            .load(Ordering::Relaxed)
    }
}
