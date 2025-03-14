// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct NetworkStats {
    // for now just experiment with basic data, we could always extend it
    active_ingress_mixnet_connections: AtomicUsize,

    active_ingress_websocket_connections: AtomicUsize,

    // the reason for additional `Arc` on this one is that the handler wasn't
    // designed with metrics in mind and this single counter has been woven through
    // the call stack
    active_egress_mixnet_connections: Arc<AtomicUsize>,
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

    pub fn new_ingress_websocket_client(&self) {
        self.active_ingress_websocket_connections
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn disconnected_ingress_websocket_client(&self) {
        self.active_ingress_websocket_connections
            .fetch_sub(1, Ordering::Relaxed);
    }

    pub fn active_ingress_mixnet_connections_count(&self) -> usize {
        self.active_ingress_mixnet_connections
            .load(Ordering::Relaxed)
    }

    pub fn active_ingress_websocket_connections_count(&self) -> usize {
        self.active_ingress_websocket_connections
            .load(Ordering::SeqCst)
    }

    pub fn active_egress_mixnet_connections_counter(&self) -> Arc<AtomicUsize> {
        self.active_egress_mixnet_connections.clone()
    }

    pub fn active_egress_mixnet_connections_count(&self) -> usize {
        self.active_egress_mixnet_connections
            .load(Ordering::Relaxed)
    }
}
