//! # API Connection statistics
//!
//! Metrics collected by the client while attempting to pull config from the API.

use super::MetricsEvents;
use std::collections::VecDeque;

use nym_metrics::{inc, inc_by};

#[derive(Default, Debug, Clone)]
struct APIMetrics {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,
}

impl APIMetrics {
    fn handle(&mut self, event: APIMetricsEvent) {
        match event {
            APIMetricsEvent::RealPacketSent(packet_size) => {
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
pub(crate) enum APIMetricsEvent {
    // The real packets sent. Recall that acks are sent by the API, so it's not included here.
    RealPacketSent(usize),
}

impl Into<MetricsEvents> for APIMetricsEvent {
    fn into(self) -> MetricsEvents {
        MetricsEvents::APIMetricsEvent(self)
    }
}

pub(crate) struct APIMetricsControl {
    // Keep track of packet statistics over time
    stats: APIMetrics,

    failures: VecDeque<()>, // TODO
}

impl super::MetricsObj for APIMetricsControl {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            stats: APIMetrics::default(),
            failures: VecDeque::new(),
        }
    }

    fn type_identity(&self) -> super::MetricsType {
        super::MetricsType::APIMetrics
    }

    fn handle_event(&mut self, event: MetricsEvents) {
        match event {
            MetricsEvents::APIMetricsEvent(ev) => self.stats.handle(ev),
            _ => log::error!("Received unusable event: {:?}", event.metrics_type()),
        }
    }

    fn snapshot(&mut self) {
        // pass
    }

    fn periodic_reset(&mut self) {
        self.stats = APIMetrics::default();
    }
}

impl super::MetricsReporter for APIMetricsControl {
    fn marshall(&self) -> std::io::Result<String> {
        self.check_for_notable_events();
        self.report_counters();
        Ok(format!("{:?}", self.stats))
    }
}

impl APIMetricsControl {
    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }

    fn check_for_notable_events(&self) {}
}
