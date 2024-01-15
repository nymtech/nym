use std::time::Duration;

use crate::spawn_future;

#[derive(Default, Debug)]
struct PacketStatistics {
    // Sent
    real_packets_sent: u64,
    cover_packets_sent: u64,

    // Received
    total_packets_received: u64,
    total_acks_received: u64,
    real_acks_received: u64,

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
            PacketStatisticsEvent::RealPacketSent => {
                self.real_packets_sent += 1;
            }
            PacketStatisticsEvent::CoverPacketSent => {
                self.cover_packets_sent += 1;
            }
            PacketStatisticsEvent::PacketReceived(packets) => {
                // self.total_packets_received += TryInto::<u64>::try_into(packets).unwrap_or(1);
                self.total_packets_received += match packets.try_into() {
                    Ok(p) => p,
                    Err(_err) => {
                        log::error!("Conversion error usize -> u64 when handling the number of received packets!");
                        1
                    }
                }
            }
            PacketStatisticsEvent::AckReceived => {
                self.total_acks_received += 1;
            }
            PacketStatisticsEvent::RealAckReceived => {
                self.real_acks_received += 1;
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
    // The real packets sent
    RealPacketSent,
    // The cover packets sent
    CoverPacketSent,

    // Packet of any type received
    PacketReceived(usize),

    // Ack of any type received
    AckReceived,
    // Out of the total acks received, this is the subset of those that were real
    RealAckReceived,

    // Types of packets queued
    RealPacketQueued,
    RetransmissionQueued,
    ReplySurbRequestQueued,
    AdditionalReplySurbRequestQueued,
}

pub(crate) type PacketStatisticsReporter =
    tokio::sync::mpsc::UnboundedSender<PacketStatisticsEvent>;
type PacketStatisticsReceiver = tokio::sync::mpsc::UnboundedReceiver<PacketStatisticsEvent>;

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
            stats_tx,
        )
    }

    fn report_statistics(&self) {
        log::info!(
            "packets sent: {} (real: {}, cover: {}, retransmissions: {})",
            self.stats.real_packets_sent + self.stats.cover_packets_sent,
            self.stats.real_packets_sent,
            self.stats.cover_packets_sent,
            self.stats.retransmissions_queued,
        );
        log::info!(
            "packets received: {}, acks received: {} (real: {}, cover: {})",
            self.stats.total_packets_received,
            self.stats.total_acks_received,
            self.stats.real_acks_received,
            self.stats.total_acks_received - self.stats.real_acks_received,
        );
    }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started PacketStatisticsControl with graceful shutdown support");

        let interval = Duration::from_secs(2);
        let mut interval = tokio::time::interval(interval);

        loop {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("PacketStatisticsControl: Received shutdown");
                    break;
                }
                Some(stats_event) = self.stats_rx.recv() => {
                    log::trace!("PacketStatisticsControl: Received stats event");
                    self.stats.handle_event(stats_event);
                }
                _ = interval.tick() => {
                    self.report_statistics();
                }
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
