//! # Gateway Connection statistics
//!
//! Metrics collected by the client while establishing and maintaining connections to the gateway.

use super::ClientStatsEvents;
use std::collections::VecDeque;

use nym_metrics::{inc, inc_by};

#[derive(Default, Debug, Clone)]
struct GatewayStats {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,
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

    /// failed connection statistics
    failures: VecDeque<()>, // TODO
}

impl super::ClientStatsObj for GatewayStatsControl {
    fn type_identity(&self) -> super::ClientStatsType {
        super::ClientStatsType::Gateway
    }

    fn handle_event(&mut self, event: ClientStatsEvents) {
        match event {
            ClientStatsEvents::GatewayConn(ev) => self.stats.handle(ev),
            _ => log::error!("Received unusable event: {:?}", event.metrics_type()),
        }
    }

    fn snapshot(&mut self) {
        // pass
    }

    fn periodic_reset(&mut self) {
        self.stats = GatewayStats::default();
    }
}

impl super::StatisticsReporter for GatewayStatsControl {
    fn marshall(&self) -> std::io::Result<String> {
        self.check_for_notable_events();
        self.report_counters();
        Ok(format!("{:?}", self.stats))
    }
}

impl GatewayStatsControl {
    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }

    fn check_for_notable_events(&self) {}
}
