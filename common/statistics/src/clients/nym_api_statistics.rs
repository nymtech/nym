// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # API Connection statistics
//!
//! Metrics collected by the client while attempting to pull config from the API.

use super::ClientStatsEvents;
use std::collections::VecDeque;

use nym_metrics::{inc, inc_by};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NymApiStats {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,

    /// API connection failure statistics
    failures: VecDeque<()>, // TODO
}

impl NymApiStats {
    fn handle(&mut self, event: NymApiStatsEvent) {
        match event {
            NymApiStatsEvent::RealPacketSent(packet_size) => {
                self.real_packets_sent += 1;
                self.real_packets_sent_size += packet_size;
                inc!("real_packets_sent");
                inc_by!("real_packets_sent_size", packet_size);
            }
        }
    }
}

/// Event space for Nym API statistics tracking
#[derive(Debug, Clone)]
pub enum NymApiStatsEvent {
    /// The real packets sent. Recall that acks are sent by the Api, so it's not included here.
    RealPacketSent(usize),
}

impl From<NymApiStatsEvent> for ClientStatsEvents {
    fn from(event: NymApiStatsEvent) -> ClientStatsEvents {
        ClientStatsEvents::NymApi(event)
    }
}

/// Nym API statistics tracking object.
///
// Internally this uses two copies since we have separate reporting epochs for local and 
// non-local stats reporting. In the future these may track different things.
#[derive(Default)]
pub struct NymApiStatsControl {
    // Keep track of packet statistics over time
    stats: NymApiStats,
}

impl NymApiStatsControl {
    pub(crate) fn handle_event(&mut self, event: NymApiStatsEvent) {
        self.stats.handle(event)
    }

    pub(crate) fn report(&self) -> NymApiStats {
        self.stats.clone()
    }

    pub(crate) fn reset(&mut self) {
        self.stats = NymApiStats::default();
    }
}
