use super::ClientStatsEvents;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use nym_metrics::{inc, inc_by};
use serde::{Deserialize, Serialize};
use si_scale::helpers::bibytes2;

// When computing rates, we include snapshots that are up to this old. We set it to some odd number
// a tad larger than an integer number of snapshot intervals, so that we don't have to worry about
// threshold effects.
// Also, set it larger than the packet report interval so that we don't miss notable singular events
const RECORDING_WINDOW_MS: u64 = 2300;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PacketStatistics {
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
    fn handle(&mut self, event: PacketStatisticsEvent) {
        match event {
            PacketStatisticsEvent::RealPacketSent(packet_size) => {
                self.real_packets_sent += 1;
                self.real_packets_sent_size += packet_size;
                inc!("real_packets_sent");
                inc_by!("real_packets_sent_size", packet_size);
            }
            PacketStatisticsEvent::CoverPacketSent(packet_size) => {
                self.cover_packets_sent += 1;
                self.cover_packets_sent_size += packet_size;
                inc!("cover_packets_sent");
                inc_by!("cover_packets_sent_size", packet_size);
            }
            PacketStatisticsEvent::RealPacketReceived(packet_size) => {
                self.real_packets_received += 1;
                self.real_packets_received_size += packet_size;
                inc!("real_packets_received");
                inc_by!("real_packets_received_size", packet_size);
            }
            PacketStatisticsEvent::CoverPacketReceived(packet_size) => {
                self.cover_packets_received += 1;
                self.cover_packets_received_size += packet_size;
                inc!("cover_packets_received");
                inc_by!("cover_packets_received_size", packet_size);
            }
            PacketStatisticsEvent::AckReceived(packet_size) => {
                self.total_acks_received += 1;
                self.total_acks_received_size += packet_size;
                inc!("total_acks_received");
                inc_by!("total_acks_received_size", packet_size);
            }
            PacketStatisticsEvent::RealAckReceived(packet_size) => {
                self.real_acks_received += 1;
                self.real_acks_received_size += packet_size;
                inc!("real_acks_received");
                inc_by!("real_acks_received_size", packet_size);
            }
            PacketStatisticsEvent::CoverAckReceived(packet_size) => {
                self.cover_acks_received += 1;
                self.cover_acks_received_size += packet_size;
                inc!("cover_acks_received");
                inc_by!("cover_acks_received_size", packet_size);
            }
            PacketStatisticsEvent::RealPacketQueued => {
                self.real_packets_queued += 1;
                inc!("real_packets_queued");
            }
            PacketStatisticsEvent::RetransmissionQueued => {
                self.retransmissions_queued += 1;
                inc!("retransmissions_queued");
            }
            PacketStatisticsEvent::ReplySurbRequestQueued => {
                self.reply_surbs_queued += 1;
                inc!("reply_surbs_queued");
            }
            PacketStatisticsEvent::AdditionalReplySurbRequestQueued => {
                self.additional_reply_surbs_queued += 1;
                inc!("additional_reply_surbs_queued");
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

pub struct MixnetBandwidthStatisticsEvent {
    pub rates: PacketRates,
}

impl MixnetBandwidthStatisticsEvent {
    pub fn new(rates: PacketRates) -> Self {
        Self { rates }
    }
}

impl nym_task::TaskStatusEvent for MixnetBandwidthStatisticsEvent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl fmt::Display for MixnetBandwidthStatisticsEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rates.summary())
    }
}

#[derive(Debug, Clone)]
pub struct PacketRates {
    pub real_packets_sent: f64,
    pub real_packets_sent_size: f64,
    pub cover_packets_sent: f64,
    pub cover_packets_sent_size: f64,

    pub real_packets_received: f64,
    pub real_packets_received_size: f64,
    pub cover_packets_received: f64,
    pub cover_packets_received_size: f64,

    pub total_acks_received: f64,
    pub total_acks_received_size: f64,
    pub real_acks_received: f64,
    pub real_acks_received_size: f64,
    pub cover_acks_received: f64,
    pub cover_acks_received_size: f64,

    pub real_packets_queued: f64,
    pub retransmissions_queued: f64,
    pub reply_surbs_queued: f64,
    pub additional_reply_surbs_queued: f64,
}

impl fmt::Display for PacketRates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "down: {}/s, up: {}/s (cover down: {}/s, cover up: {}/s)",
            bibytes2(self.real_packets_received_size),
            bibytes2(self.real_packets_sent_size),
            bibytes2(self.cover_packets_received_size),
            bibytes2(self.cover_packets_sent_size),
        )
    }
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
            "down: {}/s, up: {}/s (cover down: {}/s, cover up: {}/s)",
            bibytes2(self.real_packets_received_size),
            bibytes2(self.real_packets_sent_size),
            bibytes2(self.cover_packets_received_size),
            bibytes2(self.cover_packets_sent_size),
        )
    }

    fn detailed_summary(&self) -> String {
        format!(
            "RX: {:.1} mixpkt/s, {}/s (real: {}/s, acks: {}/s), TX: {:.1} mixpkt/s, {}/s (real: {}/s)",
            self.real_packets_received + self.cover_packets_received,
            bibytes2(self.real_packets_received_size + self.cover_packets_received_size),
            bibytes2(self.real_packets_received_size),
            bibytes2(self.total_acks_received_size),
            self.real_packets_sent + self.cover_packets_sent,
            bibytes2(self.real_packets_sent_size + self.cover_packets_sent_size),
            bibytes2(self.real_packets_sent_size),
        )
    }
}

/// Event Space used for counting the Packet types used in a connection.
#[derive(Debug)]
pub enum PacketStatisticsEvent {
    /// The real packets sent. Recall that acks are sent by the gateway, so it's not included here.
    RealPacketSent(usize),
    /// The cover packets sent
    CoverPacketSent(usize),

    /// Real packets received
    RealPacketReceived(usize),
    /// Cover packets received
    CoverPacketReceived(usize),

    /// Ack of any type received. This is mostly used as a consistency check, and should be the sum
    /// of real and cover acks received.
    AckReceived(usize),
    /// Out of the total acks received, this is the subset of those that were real
    RealAckReceived(usize),
    /// Out of the total acks received, this is the subset of those that were for cover traffic
    CoverAckReceived(usize),

    /// Types of packets queued
    RealPacketQueued,
    /// Types of packets queued
    RetransmissionQueued,
    /// Types of packets queued
    ReplySurbRequestQueued,
    /// Types of packets queued
    AdditionalReplySurbRequestQueued,
}

impl From<PacketStatisticsEvent> for ClientStatsEvents {
    fn from(event: PacketStatisticsEvent) -> ClientStatsEvents {
        ClientStatsEvents::PacketStatistics(event)
    }
}

/// Statistics tracking for Packet based I/O
#[derive(Default)]
pub struct PacketStatisticsControl {
    // Keep track of packet statistics over time
    stats: PacketStatistics,

    // We keep snapshots of the statistics over time so we can compute rates, and also keeping the
    // full history allows for some more fancy averaging if we want to do that.
    history: VecDeque<(Instant, PacketStatistics)>,

    // Keep previous rates so that we can detect notable events
    rates: VecDeque<(Instant, PacketRates)>,
}

impl PacketStatisticsControl {
    pub(crate) fn handle_event(&mut self, event: PacketStatisticsEvent) {
        self.stats.handle(event)
    }

    pub(crate) fn snapshot(&mut self) {
        self.update_history();
        self.update_rates();
    }

    pub(crate) fn report(&self) -> PacketStatistics {
        self.report_rates();
        self.check_for_notable_events();
        self.report_counters();
        self.stats.clone()
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
        // stats. Currently it should not make much of a difference since we call this just after
        // updating the history, but it seems like it could be more internally consistent to do it
        // that way.

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

    fn report_rates(&self) -> Option<PacketRates> {
        if let Some((_, rates)) = self.rates.back() {
            log::debug!("{}", rates.summary());
            log::debug!("{}", rates.detailed_summary());
            return Some(rates.clone());
        }
        None
    }

    fn report_counters(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        let (summary_sent, summary_recv) = self.stats.summary();
        log::debug!("{}", summary_sent);
        log::debug!("{}", summary_recv);
    }

    fn check_for_notable_events(&self) {
        let Some((_, latest_rates)) = self.rates.back() else {
            return;
        };

        // If we get a burst of retransmissions
        // TODO: consider making this the number of retransmissions since last report instead.
        if latest_rates.retransmissions_queued > 0.0 {
            log::debug!(
                "retransmissions: {:.2} pkt/s",
                latest_rates.retransmissions_queued
            );

            // Check what the number of retransmissions was during the recording window
            if let Some((_, start_stats)) = self.history.front() {
                let delta = self.stats.clone() - start_stats.clone();
                log::debug!(
                    "mix packet retransmissions/real mix packets: {}/{}",
                    delta.retransmissions_queued,
                    delta.real_packets_queued,
                );
            } else {
                log::warn!("Unable to check retransmissions during recording window");
            }
        }

        // IDEA: if there is a burst of acks, that could indicate tokio task starvation.
    }
}
