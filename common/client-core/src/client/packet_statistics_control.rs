use std::time::Duration;

use crate::spawn_future;

// Time interval between reporting packet statistics
const PACKET_REPORT_INTERVAL_SECS: u64 = 2;

#[derive(Default, Debug)]
struct PacketStatistics {
    // Sent
    real_packets_sent: u64,
    real_packets_sent_size: usize,
    cover_packets_sent: u64,
    cover_packets_sent_size: usize,
    background_cover_packets_sent: u64,
    background_cover_packets_sent_size: usize,

    // Received
    real_packets_received: u64,
    cover_packets_received: u64,

    // Acks
    total_acks_received: u64,
    real_acks_received: u64,
    cover_acks_received: u64,

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
            PacketStatisticsEvent::BackgroundCoverPacketSent(packet_size) => {
                self.background_cover_packets_sent += 1;
                self.background_cover_packets_sent_size += packet_size;
            }
            PacketStatisticsEvent::RealPacketReceived => {
                self.real_packets_received += 1;
            }
            PacketStatisticsEvent::CoverPacketReceived => {
                self.cover_packets_received += 1;
            }
            PacketStatisticsEvent::AckReceived => {
                self.total_acks_received += 1;
            }
            PacketStatisticsEvent::RealAckReceived => {
                self.real_acks_received += 1;
            }
            PacketStatisticsEvent::CoverAckReceived => {
                self.cover_acks_received += 1;
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
}

pub(crate) enum PacketStatisticsEvent {
    // The real packets sent. Recall that acks are sent by the gateway, so it's not included here.
    RealPacketSent(usize),
    // The cover packets sent
    CoverPacketSent(usize),
    // The cover packets sent in the background
    BackgroundCoverPacketSent(usize),

    // Real packets received
    RealPacketReceived,
    // Cover packets received
    CoverPacketReceived,

    // Ack of any type received. This is mostly used as a consistency check, and should be the sum
    // of real and cover acks received.
    AckReceived,
    // Out of the total acks received, this is the subset of those that were real
    RealAckReceived,
    // Out of the total acks received, this is the subset of those that were for cover traffic
    CoverAckReceived,

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
    stats: PacketStatistics,
}

impl PacketStatisticsControl {
    pub(crate) fn new() -> (Self, PacketStatisticsReporter) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            Self {
                stats_rx,
                stats: PacketStatistics::default(),
            },
            PacketStatisticsReporter::new(stats_tx),
        )
    }

    fn report_statistics(&self) {
        log::trace!("packet statistics: {:?}", &self.stats);
        log::info!(
            "packets sent: {} (real: {}, cover: {}, retransmissions: {})",
            self.stats.real_packets_sent + self.stats.cover_packets_sent,
            self.stats.real_packets_sent,
            self.stats.cover_packets_sent,
            self.stats.retransmissions_queued,
        );
        log::info!(
            "packets received: {}, (real: {}, cover: {}, acks: {}, acks for cover: {})",
            self.stats.real_packets_received + self.stats.cover_packets_received,
            self.stats.real_packets_received,
            self.stats.cover_packets_received,
            self.stats.real_acks_received,
            self.stats.cover_acks_received,
        );
    }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started PacketStatisticsControl with graceful shutdown support");

        let interval = Duration::from_secs(PACKET_REPORT_INTERVAL_SECS);
        let mut interval = tokio::time::interval(interval);

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
                _ = interval.tick() => {
                    self.report_statistics();
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
