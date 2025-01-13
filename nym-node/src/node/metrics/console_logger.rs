// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use human_repr::HumanCount;
use human_repr::HumanThroughput;
use nym_node_metrics::NymNodeMetrics;
use nym_task::ShutdownToken;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::task::JoinHandle;
use tokio::time::{interval_at, Instant};
use tracing::{info, trace};

struct AtLastUpdate {
    time: OffsetDateTime,

    // INGRESS
    forward_hop_packets_received: usize,

    // INGRESS
    final_hop_packets_received: usize,

    // EGRESS
    forward_hop_packets_sent: usize,

    // EGRESS
    ack_packets_sent: usize,

    wg_tx: usize,
    wg_rx: usize,
}

impl AtLastUpdate {
    fn new() -> Self {
        Self {
            time: OffsetDateTime::now_utc(),
            forward_hop_packets_received: 0,
            final_hop_packets_received: 0,
            forward_hop_packets_sent: 0,
            ack_packets_sent: 0,
            wg_tx: 0,
            wg_rx: 0,
        }
    }
}

// replicate behaviour from old mixnode to log number of mixed packets
pub(crate) struct ConsoleLogger {
    logging_delay: Duration,
    at_last_update: AtLastUpdate,
    metrics: NymNodeMetrics,
    shutdown: ShutdownToken,
}

impl ConsoleLogger {
    pub(crate) fn new(
        logging_delay: Duration,
        metrics: NymNodeMetrics,
        shutdown: ShutdownToken,
    ) -> Self {
        ConsoleLogger {
            logging_delay,
            at_last_update: AtLastUpdate::new(),
            metrics,
            shutdown,
        }
    }

    async fn log_running_stats(&mut self) {
        let now = OffsetDateTime::now_utc();
        let delta_secs = (now - self.at_last_update.time).as_seconds_f64();

        let forward_received = self.metrics.mixnet.ingress.forward_hop_packets_received();
        let final_received = self.metrics.mixnet.ingress.final_hop_packets_received();
        let forward_sent = self.metrics.mixnet.egress.forward_hop_packets_sent();
        let acks = self.metrics.mixnet.egress.ack_packets_sent();

        let wg_tx = self.metrics.wireguard.bytes_tx();
        let wg_rx = self.metrics.wireguard.bytes_rx();

        let forward_received_rate =
            (forward_received - self.at_last_update.forward_hop_packets_received) as f64
                / delta_secs;
        let final_rate =
            (final_received - self.at_last_update.final_hop_packets_received) as f64 / delta_secs;
        let forward_sent_rate =
            (forward_sent - self.at_last_update.forward_hop_packets_sent) as f64 / delta_secs;
        let acks_rate = (acks - self.at_last_update.ack_packets_sent) as f64 / delta_secs;

        let wg_tx_rate = (wg_tx - self.at_last_update.wg_tx) as f64 / delta_secs;
        let wg_rx_rate = (wg_rx - self.at_last_update.wg_rx) as f64 / delta_secs;

        info!("↑↓ Packets sent [total] / sent [acks] / received [mix] / received [gw]: {} ({}) / {} ({}) / {} ({}) / {} ({})",
            forward_sent.human_count_bare(),
            forward_sent_rate.human_throughput_bare(),
            acks.human_count_bare(),
            acks_rate.human_throughput_bare(),
            forward_received.human_count_bare(),
            forward_received_rate.human_throughput_bare(),
            final_received.human_count_bare(),
            final_rate.human_throughput_bare(),
        );

        // only log wireguard if we have transmitted ANY bytes
        if self.at_last_update.wg_rx != 0 {
            info!(
                "↑↓ Wireguard tx/rx: {} ({}) / {} ({})",
                wg_tx.human_count_bytes(),
                wg_tx_rate.human_throughput_bytes(),
                wg_rx.human_count_bytes(),
                wg_rx_rate.human_throughput_bytes()
            )
        }

        self.at_last_update.time = now;
        self.at_last_update.forward_hop_packets_received = forward_received;
        self.at_last_update.final_hop_packets_received = final_received;
        self.at_last_update.forward_hop_packets_sent = forward_sent;
        self.at_last_update.ack_packets_sent = acks;
        self.at_last_update.wg_tx = wg_tx;
        self.at_last_update.wg_rx = wg_rx;

        // TODO: add websocket-client traffic
    }

    async fn run(&mut self) {
        trace!("Starting ConsoleLogger");
        let mut interval = interval_at(Instant::now() + self.logging_delay, self.logging_delay);
        loop {
            tokio::select! {
                biased;
               _ = self.shutdown.cancelled() => {
                    trace!("ConsoleLogger: Received shutdown");
                    break
                }
                _ = interval.tick() => self.log_running_stats().await,
            };
        }
        trace!("ConsoleLogger: Exiting");
    }

    pub(crate) fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}
