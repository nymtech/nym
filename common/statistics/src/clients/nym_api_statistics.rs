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

    fn summary(&self) -> (String, String) {
        (
            format!("packets sent: {}", self.real_packets_sent,),
            String::new(),
        )
    }
}

/// Event space for Nym API statistics tracking
#[derive(Debug)]
pub enum NymApiStatsEvent {
    /// The real packets sent. Recall that acks are sent by the Api, so it's not included here.
    RealPacketSent(usize),
}

impl From<NymApiStatsEvent> for ClientStatsEvents {
    fn from(event: NymApiStatsEvent) -> ClientStatsEvents {
        ClientStatsEvents::NymApi(event)
    }
}

/// Nym API statistics tracking object
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
