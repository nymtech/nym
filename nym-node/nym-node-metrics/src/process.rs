// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct NodeStats {
    pub final_hop_packets_pending_delivery: AtomicUsize,

    pub forward_hop_packets_pending_delivery: AtomicUsize,

    pub forward_hop_packets_being_delayed: AtomicUsize,

    // packets that haven't yet been delayed and are waiting for their chance
    pub packet_forwarder_queue_size: AtomicUsize,
}

impl NodeStats {
    pub fn update_final_hop_packets_pending_delivery(&self, current: usize) {
        self.final_hop_packets_pending_delivery
            .store(current, Ordering::Relaxed);
    }

    pub fn final_hop_packets_pending_delivery_count(&self) -> usize {
        self.final_hop_packets_pending_delivery
            .load(Ordering::Relaxed)
    }

    pub fn update_forward_hop_packets_pending_delivery(&self, current: usize) {
        self.forward_hop_packets_pending_delivery
            .store(current, Ordering::Relaxed);
    }

    pub fn forward_hop_packets_pending_delivery_count(&self) -> usize {
        self.forward_hop_packets_pending_delivery
            .load(Ordering::Relaxed)
    }

    pub fn update_forward_hop_packets_being_delayed(&self, current: usize) {
        self.forward_hop_packets_being_delayed
            .store(current, Ordering::Relaxed);
    }

    pub fn forward_hop_packets_being_delayed_count(&self) -> usize {
        self.forward_hop_packets_being_delayed
            .load(Ordering::Relaxed)
    }

    pub fn update_packet_forwarder_queue_size(&self, current: usize) {
        self.packet_forwarder_queue_size
            .store(current, Ordering::Relaxed);
    }

    pub fn packet_forwarder_queue_size(&self) -> usize {
        self.packet_forwarder_queue_size.load(Ordering::Relaxed)
    }
}
