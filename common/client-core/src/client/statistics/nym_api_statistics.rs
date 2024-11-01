//! # API Connection statistics
//!
//! Metrics collected by the client while attempting to pull config from the API.

use super::StatsEvents;
use std::collections::VecDeque;

use nym_metrics::{inc, inc_by};

#[derive(Default, Debug, Clone)]
struct NymApiStats {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,
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

#[derive(Debug)]
pub(crate) enum NymApiStatsEvent {
    // The real packets sent. Recall that acks are sent by the Api, so it's not included here.
    RealPacketSent(usize),
}

impl From<NymApiStatsEvent> for StatsEvents {
    fn from(event: NymApiStatsEvent) -> StatsEvents {
        StatsEvents::NymApi(event)
    }
}

pub(crate) struct NymApiStatsControl {
    // Keep track of packet statistics over time
    stats: NymApiStats,

    failures: VecDeque<()>, // TODO
}

impl super::StatsObj for NymApiStatsControl {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            stats: NymApiStats::default(),
            failures: VecDeque::new(),
        }
    }

    fn type_identity(&self) -> super::StatsType {
        super::StatsType::NymApi
    }

    fn handle_event(&mut self, event: StatsEvents) {
        match event {
            StatsEvents::NymApi(ev) => self.stats.handle(ev),
            _ => log::error!("Received unusable event: {:?}", event.metrics_type()),
        }
    }

    fn snapshot(&mut self) {
        // pass
    }

    fn periodic_reset(&mut self) {
        self.stats = NymApiStats::default();
    }
}

impl super::StatisticsReporter for NymApiStatsControl {
    fn marshall(&self) -> std::io::Result<String> {
        self.check_for_notable_events();
        self.report_counters();
        Ok(format!("{:?}", self.stats))
    }
}

impl NymApiStatsControl {
    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }

    fn check_for_notable_events(&self) {}
}
