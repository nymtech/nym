use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use si_scale::helpers::bibytes2;

use crate::spawn_future;

// Time interval between reporting packet statistics
const PACKET_REPORT_INTERVAL_SECS: u64 = 2;
// Interval for taking snapshots of the packet statistics
const SNAPSHOT_INTERVAL_MS: u64 = 500;
// When computing rates, we include snapshots that are up to this old. We set it to some odd number
// a tad larger than an integer number of snapshot intervals, so that we don't have to worry about
// threshold effects
const RECORDING_WINDOW_MS: u64 = 1700;

#[derive(Default, Debug, Clone)]
struct PacketStatistics {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,
    cover_packets_sent: u64,
    cover_packets_sent_size: usize,

    // Received
    real_packets_received: u64,
    real_packets_received_size: usize,
    cover_packets_received: u64,
    cover_packets_received_size: usize,

    // Acks
    total_acks_received: u64,
    total_acks_received_size: usize,
    real_acks_received: u64,
    real_acks_received_size: usize,
    cover_acks_received: u64,
    cover_acks_received_size: usize,

    // Types of packets queued
    // TODO: track the type sent instead
    real_packets_queued: u64,
    retransmissions_queued: u64,
    reply_surbs_queued: u64,
    additional_reply_surbs_queued: u64,
}

impl PacketStatistics {
    fn handle_event(&mut self, event: PacketStatisticsEvent) {
        match event {
            PacketStatisticsEvent::RealPacketSent(packet_size) => {
                self.real_packets_sent += 1;
                self.real_packets_sent_size += packet_size;
            }
            PacketStatisticsEvent::CoverPacketSent(packet_size) => {
                self.cover_packets_sent += 1;
                self.cover_packets_sent_size += packet_size;
            }
            PacketStatisticsEvent::RealPacketReceived(packet_size) => {
                self.real_packets_received += 1;
                self.real_packets_received_size += packet_size;
            }
            PacketStatisticsEvent::CoverPacketReceived(packet_size) => {
                self.cover_packets_received += 1;
                self.cover_packets_received_size += packet_size;
            }
            PacketStatisticsEvent::AckReceived(packet_size) => {
                self.total_acks_received += 1;
                self.total_acks_received_size += packet_size;
            }
            PacketStatisticsEvent::RealAckReceived(packet_size) => {
                self.real_acks_received += 1;
                self.real_acks_received_size += packet_size;
            }
            PacketStatisticsEvent::CoverAckReceived(packet_size) => {
                self.cover_acks_received += 1;
                self.cover_acks_received_size += packet_size;
            }
            PacketStatisticsEvent::RealPacketQueued => {
                self.real_packets_queued += 1;
            }
            PacketStatisticsEvent::RetransmissionQueued => {
                self.retransmissions_queued += 1;
            }
            PacketStatisticsEvent::ReplySurbRequestQueued => {
                self.reply_surbs_queued += 1;
            }
            PacketStatisticsEvent::AdditionalReplySurbRequestQueued => {
                self.additional_reply_surbs_queued += 1;
            }
        }
    }

    fn summary(&self) -> (String, String) {
        (
            format!(
                "packets sent: {} (real: {}, cover: {}, retransmissions: {})",
                self.real_packets_sent + self.cover_packets_sent,
                self.real_packets_sent,
                self.cover_packets_sent,
                self.retransmissions_queued,
            ),
            format!(
                "packets received: {}, (real: {}, cover: {}, acks: {}, acks for cover: {})",
                self.real_packets_received + self.cover_packets_received,
                self.real_packets_received,
                self.cover_packets_received,
                self.real_acks_received,
                self.cover_acks_received,
            ),
        )
    }
}

impl std::ops::Sub for PacketStatistics {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            real_packets_sent: self.real_packets_sent - rhs.real_packets_sent,
            real_packets_sent_size: self.real_packets_sent_size - rhs.real_packets_sent_size,
            cover_packets_sent: self.cover_packets_sent - rhs.cover_packets_sent,
            cover_packets_sent_size: self.cover_packets_sent_size - rhs.cover_packets_sent_size,

            real_packets_received: self.real_packets_received - rhs.real_packets_received,
            real_packets_received_size: self.real_packets_received_size
                - rhs.real_packets_received_size,
            cover_packets_received: self.cover_packets_received - rhs.cover_packets_received,
            cover_packets_received_size: self.cover_packets_received_size
                - rhs.cover_packets_received_size,

            total_acks_received: self.total_acks_received - rhs.total_acks_received,
            total_acks_received_size: self.total_acks_received_size - rhs.total_acks_received_size,
            real_acks_received: self.real_acks_received - rhs.real_acks_received,
            real_acks_received_size: self.real_acks_received_size - rhs.real_acks_received_size,
            cover_acks_received: self.cover_acks_received - rhs.cover_acks_received,
            cover_acks_received_size: self.cover_acks_received_size - rhs.cover_acks_received_size,

            real_packets_queued: self.real_packets_queued - rhs.real_packets_queued,
            retransmissions_queued: self.retransmissions_queued - rhs.retransmissions_queued,
            reply_surbs_queued: self.reply_surbs_queued - rhs.reply_surbs_queued,
            additional_reply_surbs_queued: self.additional_reply_surbs_queued
                - rhs.additional_reply_surbs_queued,
        }
    }
}

#[derive(Debug, Clone)]
struct PacketRates {
    real_packets_sent: f64,
    real_packets_sent_size: f64,
    cover_packets_sent: f64,
    cover_packets_sent_size: f64,

    real_packets_received: f64,
    real_packets_received_size: f64,
    cover_packets_received: f64,
    cover_packets_received_size: f64,

    total_acks_received: f64,
    total_acks_received_size: f64,
    real_acks_received: f64,
    real_acks_received_size: f64,
    cover_acks_received: f64,
    cover_acks_received_size: f64,

    real_packets_queued: f64,
    retransmissions_queued: f64,
    reply_surbs_queued: f64,
    additional_reply_surbs_queued: f64,
}

impl From<PacketStatistics> for PacketRates {
    fn from(stats: PacketStatistics) -> Self {
        Self {
            real_packets_sent: stats.real_packets_sent as f64,
            real_packets_sent_size: stats.real_packets_sent_size as f64,
            cover_packets_sent: stats.cover_packets_sent as f64,
            cover_packets_sent_size: stats.cover_packets_sent_size as f64,

            real_packets_received: stats.real_packets_received as f64,
            real_packets_received_size: stats.real_packets_received_size as f64,
            cover_packets_received: stats.cover_packets_received as f64,
            cover_packets_received_size: stats.cover_packets_received_size as f64,

            total_acks_received: stats.total_acks_received as f64,
            total_acks_received_size: stats.total_acks_received_size as f64,
            real_acks_received: stats.real_acks_received as f64,
            real_acks_received_size: stats.real_acks_received_size as f64,
            cover_acks_received: stats.cover_acks_received as f64,
            cover_acks_received_size: stats.cover_acks_received_size as f64,

            real_packets_queued: stats.real_packets_queued as f64,
            retransmissions_queued: stats.retransmissions_queued as f64,
            reply_surbs_queued: stats.reply_surbs_queued as f64,
            additional_reply_surbs_queued: stats.additional_reply_surbs_queued as f64,
        }
    }
}

impl std::ops::Sub for PacketRates {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            real_packets_sent: self.real_packets_sent - rhs.real_packets_sent,
            real_packets_sent_size: self.real_packets_sent_size - rhs.real_packets_sent_size,
            cover_packets_sent: self.cover_packets_sent - rhs.cover_packets_sent,
            cover_packets_sent_size: self.cover_packets_sent_size - rhs.cover_packets_sent_size,

            real_packets_received: self.real_packets_received - rhs.real_packets_received,
            real_packets_received_size: self.real_packets_received_size
                - rhs.real_packets_received_size,
            cover_packets_received: self.cover_packets_received - rhs.cover_packets_received,
            cover_packets_received_size: self.cover_packets_received_size
                - rhs.cover_packets_received_size,

            total_acks_received: self.total_acks_received - rhs.total_acks_received,
            total_acks_received_size: self.total_acks_received_size - rhs.total_acks_received_size,
            real_acks_received: self.real_acks_received - rhs.real_acks_received,
            real_acks_received_size: self.real_acks_received_size - rhs.real_acks_received_size,
            cover_acks_received: self.cover_acks_received - rhs.cover_acks_received,
            cover_acks_received_size: self.cover_acks_received_size - rhs.cover_acks_received_size,

            real_packets_queued: self.real_packets_queued - rhs.real_packets_queued,
            retransmissions_queued: self.retransmissions_queued - rhs.retransmissions_queued,
            reply_surbs_queued: self.reply_surbs_queued - rhs.reply_surbs_queued,
            additional_reply_surbs_queued: self.additional_reply_surbs_queued
                - rhs.additional_reply_surbs_queued,
        }
    }
}

impl std::ops::Div<f64> for PacketRates {
    type Output = Self;

    fn div(self, rhs: f64) -> Self::Output {
        Self {
            real_packets_sent: self.real_packets_sent / rhs,
            real_packets_sent_size: self.real_packets_sent_size / rhs,
            cover_packets_sent: self.cover_packets_sent / rhs,
            cover_packets_sent_size: self.cover_packets_sent_size / rhs,

            real_packets_received: self.real_packets_received / rhs,
            real_packets_received_size: self.real_packets_received_size / rhs,
            cover_packets_received: self.cover_packets_received / rhs,
            cover_packets_received_size: self.cover_packets_received_size / rhs,

            total_acks_received: self.total_acks_received / rhs,
            total_acks_received_size: self.total_acks_received_size / rhs,
            real_acks_received: self.real_acks_received / rhs,
            real_acks_received_size: self.real_acks_received_size / rhs,
            cover_acks_received: self.cover_acks_received / rhs,
            cover_acks_received_size: self.cover_acks_received_size / rhs,

            real_packets_queued: self.real_packets_queued / rhs,
            retransmissions_queued: self.retransmissions_queued / rhs,
            reply_surbs_queued: self.reply_surbs_queued / rhs,
            additional_reply_surbs_queued: self.additional_reply_surbs_queued / rhs,
        }
    }
}

impl PacketRates {
    fn summary(&self) -> String {
        format!(
            "rx: {}/s (real: {}/s, acks: {}/s), tx: {}/s (real: {}/s)",
            bibytes2(self.real_packets_received_size + self.cover_packets_received_size),
            bibytes2(self.real_packets_received_size),
            bibytes2(self.total_acks_received_size),
            bibytes2(self.real_packets_sent_size + self.cover_packets_sent_size),
            bibytes2(self.real_packets_sent_size),
        )
    }
}

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

    // Keep previous rates so that we can detect notable events
    rates: VecDeque<(Instant, PacketRates)>,
}

impl PacketStatisticsControl {
    pub(crate) fn new() -> (Self, PacketStatisticsReporter) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();

        (
            Self {
                stats_rx,
                stats: PacketStatistics::default(),
                history: VecDeque::new(),
                rates: VecDeque::new(),
            },
            PacketStatisticsReporter::new(stats_tx),
        )
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

    fn compute_rates(&self) -> Option<PacketRates> {
        // NOTE: consider changing this to compute rates over the history instead of using current
        // stats

        // Do basic averaging over the entire history, which just uses the first and last
        if let Some((start, start_stats)) = self.history.front() {
            let duration_secs = Instant::now().duration_since(*start).as_secs_f64();
            let delta = self.stats.clone() - start_stats.clone();
            let rates = PacketRates::from(delta) / duration_secs;
            Some(rates)
        } else {
            None
        }
    }

    fn update_rates(&mut self) {
        // Update latest
        if let Some(rates) = self.compute_rates() {
            self.rates.push_back((Instant::now(), rates));
        }

        // Filter out old ones
        let recording_window = Instant::now() - Duration::from_millis(RECORDING_WINDOW_MS);
        while self
            .rates
            .front()
            .map_or(false, |&(t, _)| t < recording_window)
        {
            self.rates.pop_front();
        }
    }

    fn report_rates(&self) {
        if let Some((_, rates)) = self.rates.back() {
            log::info!("{}", rates.summary());
        }
    }

    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }

    fn check_for_notable_events(&self) {
        if let Some((_, rates)) = self.rates.back() {
            // If we get a burst of retransmissions
            if rates.retransmissions_queued > 0.0 {
                log::warn!("retransmissions: {} pkt/s", rates.retransmissions_queued);
            }
            if rates.total_acks_received > 0.0 {
                log::info!("acks: {:.1} pkt/s", rates.total_acks_received);
            }

            // Check if there is a sudden increase in acks received
            if let Some((_, rates0)) = self.rates.front() {
                let delta = rates.clone() - rates0.clone();
                if delta.total_acks_received > 10.0 {
                    log::warn!(
                        "ack rate increased from {:.1} to {:.1} pkt/s",
                        rates0.total_acks_received,
                        rates.total_acks_received
                    );
                }
            }
        }
    }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started PacketStatisticsControl with graceful shutdown support");

        let interval = Duration::from_secs(PACKET_REPORT_INTERVAL_SECS);
        let mut interval = tokio::time::interval(interval);
        let snapshot_interval = Duration::from_millis(SNAPSHOT_INTERVAL_MS);
        let mut snapshot_interval = tokio::time::interval(snapshot_interval);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                    Some(stats_event) => {
                        log::trace!("PacketStatisticsControl: Received stats event");
                        self.stats.handle_event(stats_event);
                    },
                    None => {
                        log::trace!("PacketStatisticsControl: stopping since stats channel was closed");
                        break;
                    }
                },
                _ = snapshot_interval.tick() => {
                    self.update_history();
                    self.update_rates();
                }
                _ = interval.tick() => {
                    self.report_rates();
                    self.check_for_notable_events();
                    self.report_counters();
                }
                _ = shutdown.recv_with_delay() => {
                    log::trace!("PacketStatisticsControl: Received shutdown");
                    break;
                },
            }
        }
        log::debug!("PacketStatisticsControl: Exiting");
    }

    pub(crate) fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client).await;
        })
    }
}
