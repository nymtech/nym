use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use prometheus::{Counter, Encoder, TextEncoder};

use crate::{error::MetricsError, spawn_future};

// Time interval between reporting packet statistics
const PACKET_REPORT_INTERVAL_SECS: u64 = 2;
// Interval for taking snapshots of the packet statistics
const SNAPSHOT_INTERVAL_MS: u64 = 500;
// When computing rates, we include snapshots that are up to this old. We set it to some odd number
// a tad larger than an integer number of snapshot intervals, so that we don't have to worry about
// threshold effects.
// Also, set it larger than the packet report interval so that we don't miss notable singular events
const RECORDING_WINDOW_MS: u64 = 2300;

#[derive(Debug, Clone)]
struct PacketStatistics {
    // Sent
    real_packets_sent: Counter,
    real_packets_sent_size: Counter,
    cover_packets_sent: Counter,
    cover_packets_sent_size: Counter,

    // Received
    real_packets_received: Counter,
    real_packets_received_size: Counter,
    cover_packets_received: Counter,
    cover_packets_received_size: Counter,

    // Acks
    total_acks_received: Counter,
    total_acks_received_size: Counter,
    real_acks_received: Counter,
    real_acks_received_size: Counter,
    cover_acks_received: Counter,
    cover_acks_received_size: Counter,

    // Types of packets queued
    // TODO: track the type sent instead
    real_packets_queued: Counter,
    retransmissions_queued: Counter,
    reply_surbs_queued: Counter,
    additional_reply_surbs_queued: Counter,
}

impl PacketStatistics {
    fn new() -> Result<Self, MetricsError> {
        Ok(Self {
            real_packets_sent: Counter::new("real_packets_sent", "")?,
            real_packets_sent_size: Counter::new("real_packets_sent_size", "")?,
            cover_packets_sent: Counter::new("cover_packets_sent", "")?,
            cover_packets_sent_size: Counter::new("cover_packets_sent_size", "")?,

            real_packets_received: Counter::new("real_packets_received", "")?,
            real_packets_received_size: Counter::new("real_packets_received_size", "")?,
            cover_packets_received: Counter::new("cover_packets_received", "")?,
            cover_packets_received_size: Counter::new("cover_packets_received_size", "")?,

            total_acks_received: Counter::new("total_acks_received", "")?,
            total_acks_received_size: Counter::new("total_acks_received_size", "")?,
            real_acks_received: Counter::new("real_acks_received", "")?,
            real_acks_received_size: Counter::new("real_acks_received_size", "")?,
            cover_acks_received: Counter::new("cover_acks_received", "")?,
            cover_acks_received_size: Counter::new("cover_acks_received_size", "")?,

            real_packets_queued: Counter::new("real_packets_queued", "")?,
            retransmissions_queued: Counter::new("retransmissions_queued", "")?,
            reply_surbs_queued: Counter::new("reply_surbs_queued", "")?,
            additional_reply_surbs_queued: Counter::new("additional_reply_surbs_queued", "")?,
        })
    }

    fn summary(&self) -> (String, String) {
        (
            format!(
                "packets sent: {} (real: {}, cover: {}, retransmissions: {})",
                self.real_packets_sent.get() + self.cover_packets_sent.get(),
                self.real_packets_sent.get(),
                self.cover_packets_sent.get(),
                self.retransmissions_queued.get(),
            ),
            format!(
                "packets received: {}, (real: {}, cover: {}, acks: {}, acks for cover: {})",
                self.real_packets_received.get() + self.cover_packets_received.get(),
                self.real_packets_received.get(),
                self.cover_packets_received.get(),
                self.real_acks_received.get(),
                self.cover_acks_received.get(),
            ),
        )
    }
}

#[derive(Debug)]
pub(crate) enum PacketStatisticsEvent {
    // The real packets sent. Recall that acks are sent by the gateway, so it's not included here.
    RealPacketSent(usize),
    // The cover packets sent
    CoverPacketSent(usize),

    // Real packets received
    RealPacketReceived(usize),
    // Cover packets received
    CoverPacketReceived(usize),

    // Ack of any type received. This is mostly used as a consistency check, and should be the sum
    // of real and cover acks received.
    AckReceived(usize),
    // Out of the total acks received, this is the subset of those that were real
    RealAckReceived(usize),
    // Out of the total acks received, this is the subset of those that were for cover traffic
    CoverAckReceived(usize),

    // Types of packets queued
    RealPacketQueued,
    RetransmissionQueued,
    ReplySurbRequestQueued,
    AdditionalReplySurbRequestQueued,
}

type PacketStatisticsReceiver = tokio::sync::mpsc::UnboundedReceiver<PacketStatisticsEvent>;

#[derive(Clone)]
pub(crate) struct PacketStatisticsReporter {
    stats_tx: tokio::sync::mpsc::UnboundedSender<PacketStatisticsEvent>,
}

impl PacketStatisticsReporter {
    pub(crate) fn new(stats_tx: tokio::sync::mpsc::UnboundedSender<PacketStatisticsEvent>) -> Self {
        Self { stats_tx }
    }

    pub(crate) fn report(&self, event: PacketStatisticsEvent) {
        self.stats_tx.send(event).unwrap_or_else(|err| {
            log::error!("Failed to report packet stat: {:?}", err);
        });
    }
}

pub(crate) struct PacketStatisticsControl {
    // Incoming packet stats events from other tasks
    stats_rx: PacketStatisticsReceiver,

    // Keep track of packet statistics over time
    stats: PacketStatistics,

    // We keep snapshots of the statistics over time so we can compute rates, and also keeping the
    // full history allows for some more fancy averaging if we want to do that.
    history: VecDeque<(Instant, PacketStatistics)>,

    registry: prometheus::Registry,
}

impl PacketStatisticsControl {
    pub(crate) fn new() -> Result<(Self, PacketStatisticsReporter), MetricsError> {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();
        let registry = prometheus::Registry::new();
        let stats = PacketStatistics::new()?;

        registry.register(Box::new(stats.real_packets_sent.clone()))?;
        registry.register(Box::new(stats.real_packets_sent_size.clone()))?;
        registry.register(Box::new(stats.cover_packets_sent.clone()))?;
        registry.register(Box::new(stats.cover_packets_sent_size.clone()))?;

        registry.register(Box::new(stats.real_packets_received.clone()))?;
        registry.register(Box::new(stats.real_packets_received_size.clone()))?;
        registry.register(Box::new(stats.cover_packets_received.clone()))?;
        registry.register(Box::new(stats.cover_packets_received_size.clone()))?;

        registry.register(Box::new(stats.total_acks_received.clone()))?;
        registry.register(Box::new(stats.total_acks_received_size.clone()))?;
        registry.register(Box::new(stats.real_acks_received.clone()))?;
        registry.register(Box::new(stats.real_acks_received_size.clone()))?;
        registry.register(Box::new(stats.cover_acks_received.clone()))?;
        registry.register(Box::new(stats.cover_acks_received_size.clone()))?;

        registry.register(Box::new(stats.real_packets_queued.clone()))?;
        registry.register(Box::new(stats.retransmissions_queued.clone()))?;
        registry.register(Box::new(stats.reply_surbs_queued.clone()))?;
        registry.register(Box::new(stats.additional_reply_surbs_queued.clone()))?;

        Ok((
            Self {
                stats_rx,
                stats,
                history: VecDeque::new(),
                registry,
            },
            PacketStatisticsReporter::new(stats_tx),
        ))
    }

    // Add the current stats to the history, and remove old ones.
    fn update_history(&mut self) {
        // Update latest
        self.history.push_back((Instant::now(), self.stats.clone()));

        // Filter out old ones
        let recording_window = Instant::now() - Duration::from_millis(RECORDING_WINDOW_MS);
        while self
            .history
            .front()
            .map_or(false, |&(t, _)| t < recording_window)
        {
            self.history.pop_front();
        }
    }

    #[allow(dead_code)]
    fn prom(&self) -> Result<String, MetricsError> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metrics = self.registry.gather();
        encoder.encode(&metrics, &mut buffer).unwrap();
        Ok(String::from_utf8(buffer)?)
    }

    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }

    pub(crate) async fn run_with_shutdown(
        &mut self,
        mut shutdown: nym_task::TaskClient,
    ) -> Result<(), MetricsError> {
        log::debug!("Started PacketStatisticsControl with graceful shutdown support");

        let report_interval = Duration::from_secs(PACKET_REPORT_INTERVAL_SECS);
        let mut report_interval = tokio::time::interval(report_interval);
        let snapshot_interval = Duration::from_millis(SNAPSHOT_INTERVAL_MS);
        let mut snapshot_interval = tokio::time::interval(snapshot_interval);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                    Some(stats_event) => {
                        log::trace!("PacketStatisticsControl: Received stats event");
                        self.handle_event(stats_event);
                    },
                    None => {
                        log::trace!("PacketStatisticsControl: stopping since stats channel was closed");
                        break;
                    }
                },
                _ = snapshot_interval.tick() => {
                    self.update_history();
                }
                _ = report_interval.tick() => {
                    self.report_counters();
                }
                _ = shutdown.recv_with_delay() => {
                    log::trace!("PacketStatisticsControl: Received shutdown");
                    break;
                },
            }
        }
        log::debug!("PacketStatisticsControl: Exiting");
        Ok(())
    }

    pub(crate) fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client)
                .await
                .unwrap_or_else(|err| {
                    log::error!("PacketStatisticsControl: Error: {:?}", err);
                });
        })
    }

    fn handle_event(&mut self, event: PacketStatisticsEvent) {
        match event {
            PacketStatisticsEvent::RealPacketSent(packet_size) => {
                self.stats.real_packets_sent.inc();
                self.stats.real_packets_sent_size.inc_by(packet_size as f64);
            }
            PacketStatisticsEvent::CoverPacketSent(packet_size) => {
                self.stats.cover_packets_sent.inc();
                self.stats
                    .cover_packets_sent_size
                    .inc_by(packet_size as f64);
            }
            PacketStatisticsEvent::RealPacketReceived(packet_size) => {
                self.stats.real_packets_received.inc();
                self.stats
                    .real_packets_received_size
                    .inc_by(packet_size as f64);
            }
            PacketStatisticsEvent::CoverPacketReceived(packet_size) => {
                self.stats.cover_packets_received.inc();
                self.stats
                    .cover_packets_received_size
                    .inc_by(packet_size as f64);
            }
            PacketStatisticsEvent::AckReceived(packet_size) => {
                self.stats.total_acks_received.inc();
                self.stats
                    .total_acks_received_size
                    .inc_by(packet_size as f64);
            }
            PacketStatisticsEvent::RealAckReceived(packet_size) => {
                self.stats.real_acks_received.inc();
                self.stats
                    .real_acks_received_size
                    .inc_by(packet_size as f64);
            }
            PacketStatisticsEvent::CoverAckReceived(packet_size) => {
                self.stats.cover_acks_received.inc();
                self.stats
                    .cover_acks_received_size
                    .inc_by(packet_size as f64);
            }
            PacketStatisticsEvent::RealPacketQueued => {
                self.stats.real_packets_queued.inc();
            }
            PacketStatisticsEvent::RetransmissionQueued => {
                self.stats.retransmissions_queued.inc();
            }
            PacketStatisticsEvent::ReplySurbRequestQueued => {
                self.stats.reply_surbs_queued.inc();
            }
            PacketStatisticsEvent::AdditionalReplySurbRequestQueued => {
                self.stats.additional_reply_surbs_queued.inc();
            }
        }
    }
}
