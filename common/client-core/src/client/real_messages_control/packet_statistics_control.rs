use std::{sync::atomic::Ordering::Relaxed, time::Duration};

use crate::client;

pub(crate) struct PacketStatisticsControl {}

impl PacketStatisticsControl {
    pub(crate) fn new() -> Self {
        Self {}
    }

    fn report_statistics(&self) {
        let real_packets_sent = client::REAL_PACKETS_SENT.load(Relaxed);
        let cover_packets_sent = client::COVER_PACKETS_SENT.load(Relaxed);
        let real_acks_received = client::REAL_ACKS_RECEIVED.load(Relaxed);
        let total_acks_received = client::TOTAL_ACKS_RECEIVED.load(Relaxed);
        let _real_packets_queued = client::REAL_PACKETS_QUEUED.load(Relaxed);
        let retransmissions_queued = client::RETRANSMISSIONS_QUEUED.load(Relaxed);
        let _reply_surbs_queued = client::REPLY_SURB_REQUESTS_QUEUED.load(Relaxed);
        let _additional_reply_surbs_queued = client::ADDITIONAL_REPLY_SURBS_QUEUED.load(Relaxed);

        log::info!(
            "packets sent: {} (real: {}, cover: {}, retransmissions: {})",
            real_packets_sent + cover_packets_sent,
            real_packets_sent,
            cover_packets_sent,
            retransmissions_queued,
        );
        log::info!(
            "acks received: {} (real: {}, cover: {})",
            total_acks_received,
            real_acks_received,
            total_acks_received - real_acks_received,
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
                _ = interval.tick() => {
                    self.report_statistics();
                }
            }
        }
        log::debug!("PacketStatisticsControl: Exiting");
    }
}
