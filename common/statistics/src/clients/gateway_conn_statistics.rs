// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # Gateway Connection statistics
//!
//! Metrics collected by the client while establishing and maintaining connections to the gateway.

use super::ClientStatsEvents;
use std::collections::VecDeque;

use nym_metrics::{inc, inc_by};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GatewayStats {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,

    /// failed connection statistics
    failures: VecDeque<()>, // TODO
}

impl GatewayStats {
    fn handle(&mut self, event: GatewayStatsEvent) {
        match event {
            GatewayStatsEvent::RealPacketSent(packet_size) => {
                self.real_packets_sent += 1;
                self.real_packets_sent_size += packet_size;
                inc!("real_packets_sent");
                inc_by!("real_packets_sent_size", packet_size);
            }
        }
    }

    fn summary(&self) -> (String, String) {
        (
            format!("packets sent: {}", self.real_packets_sent,),
            String::new(),
        )
    }
}

impl From<GatewayStatsEvent> for ClientStatsEvents {
    fn from(event: GatewayStatsEvent) -> ClientStatsEvents {
        ClientStatsEvents::GatewayConn(event)
    }
}

/// Event space for Gateway Connection Events
#[derive(Debug)]
pub enum GatewayStatsEvent {
    /// The real packets sent. Recall that acks are sent by the gateway, so it's not included here.
    RealPacketSent(usize),
}

/// Gateway Statistics Tracking
#[derive(Default)]
pub struct GatewayStatsControl {
    // Keep track of packet statistics over time
    stats: GatewayStats,
}

impl GatewayStatsControl {
    pub(crate) fn handle_event(&mut self, event: GatewayStatsEvent) {
        self.stats.handle(event)
    }

    pub(crate) fn report(&self) -> GatewayStats {
        self.report_counters();
        self.stats.clone()
    }

    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }
}
