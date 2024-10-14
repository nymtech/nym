//! # Gateway Connection statistics
//!
//! Metrics collected by the client while establishing and maintaining connections to the gateway.

use super::MetricsEvents;
use std::collections::VecDeque;

use nym_metrics::{inc, inc_by};

#[derive(Default, Debug, Clone)]
struct GatewayMetrics {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,
}

impl GatewayMetrics {
    fn handle(&mut self, event: GatewayMetricsEvent) {
        match event {
            GatewayMetricsEvent::RealPacketSent(packet_size) => {
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

impl Into<MetricsEvents> for GatewayMetricsEvent {
    fn into(self) -> MetricsEvents {
        MetricsEvents::GatewayMetricsEvent(self)
    }
}

#[derive(Debug)]
pub(crate) enum GatewayMetricsEvent {
    // The real packets sent. Recall that acks are sent by the gateway, so it's not included here.
    RealPacketSent(usize),
}

pub(crate) struct GatewayMetricsControl {
    // Keep track of packet statistics over time
    stats: GatewayMetrics,

    failures: VecDeque<()>, // TODO
}

impl super::MetricsObj for GatewayMetricsControl {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            stats: GatewayMetrics::default(),
            failures: VecDeque::new(),
        }
    }

    fn type_identity(&self) -> super::MetricsType {
        super::MetricsType::GatewayMetrics
    }

    fn handle_event(&mut self, event: MetricsEvents) {
        match event {
            MetricsEvents::GatewayMetricsEvent(ev) => self.stats.handle(ev),
            _ => log::error!("Received unusable event: {:?}", event.metrics_type()),
        }
    }

    fn snapshot(&mut self) {
        // pass
    }

    fn periodic_reset(&mut self) {
        self.stats = GatewayMetrics::default();
    }
}

impl super::MetricsReporter for GatewayMetricsControl {
    fn marshall(&self) -> std::io::Result<String> {
        self.check_for_notable_events();
        self.report_counters();
        Ok(format!("{:?}", self.stats))
    }
}

impl GatewayMetricsControl {
    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }

    fn check_for_notable_events(&self) {}
}
