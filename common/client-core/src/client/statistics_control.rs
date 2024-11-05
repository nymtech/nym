// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! # Statistics collection and reporting.
//!
//! Modular metrics collection and reporting system. submodules can be added to collect different types of metrics.
//! On creation the Statistics controller will start a task that will listen for incoming stats events and
//! multiplex them out to the appropriate metrics module based on type.
//!
//! Adding A new module you need to write a new module that implements the `StatsObj` trait and add it to
//! the `stats` hashmap in the `StatisticsControl` struct during it's initialization in the `new` function in
//! this file.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use std::{collections::HashMap, time::Duration};

use nym_sphinx::addressing::Recipient;
use nym_statistics_common::{
    clients::{gateway_conn_statistics, nym_api_statistics, packet_statistics},
    clients::{ClientStatsObj, ClientStatsReceiver, ClientStatsSender, ClientStatsType},
    report::{ClientStatsReport, StatisticsReporter},
};
use nym_task::connections::TransmissionLane;

use crate::{
    client::inbound_messages::{InputMessage, InputMessageSender},
    spawn_future,
};

/// Time interval between reporting statistics
const STATS_REPORT_INTERVAL_SECS: u64 = 300;
/// Interval for taking snapshots of the statistics
const SNAPSHOT_INTERVAL_MS: u64 = 500;

/// Client Stats Types
enum Stats {
    Packets(packet_statistics::PacketStatisticsControl),
    GatewayConn(gateway_conn_statistics::GatewayStatsControl),
    NymApi(nym_api_statistics::NymApiStatsControl),
}

impl StatisticsReporter for Stats {
    fn marshall(&self) -> std::io::Result<String> {
        match self {
            Stats::Packets(s) => s.marshall(),
            Stats::GatewayConn(s) => s.marshall(),
            Stats::NymApi(s) => s.marshall(),
        }
    }
}

impl ClientStatsObj for Stats {
    fn handle_event(&mut self, event: nym_statistics_common::clients::ClientStatsEvents) {
        match self {
            Stats::Packets(s) => s.handle_event(event),
            Stats::GatewayConn(s) => s.handle_event(event),
            Stats::NymApi(s) => s.handle_event(event),
        }
    }

    fn periodic_reset(&mut self) {
        match self {
            Stats::Packets(s) => s.periodic_reset(),
            Stats::GatewayConn(s) => s.periodic_reset(),
            Stats::NymApi(s) => s.periodic_reset(),
        }
    }

    fn snapshot(&mut self) {
        match self {
            Stats::Packets(s) => s.snapshot(),
            Stats::GatewayConn(s) => s.snapshot(),
            Stats::NymApi(s) => s.snapshot(),
        }
    }

    fn type_identity(&self) -> ClientStatsType {
        match self {
            Stats::Packets(s) => s.type_identity(),
            Stats::GatewayConn(s) => s.type_identity(),
            Stats::NymApi(s) => s.type_identity(),
        }
    }
}

/// Launches and manages metrics collection and reporting.
///
/// This is designed to be generic to allow for multiple types of metrics to be collected and
/// reported.
pub(crate) struct StatisticsControl {
    /// Keep store the different types of metrics collectors
    stats: [(ClientStatsType, Stats); 3],

    /// Incoming packet stats events from other tasks
    stats_rx: ClientStatsReceiver,

    /// Channel to send stats report through the mixnet
    report_tx: InputMessageSender,

    /// Service-provider address to send stats reports
    reporting_address: Recipient,

    /// Report structure containing (privacy preserving) context information.
    stats_report: ClientStatsReport,
}

impl StatisticsControl {
    pub(crate) fn new(
        reporting_address: Recipient,
        report_tx: InputMessageSender,
    ) -> (Self, ClientStatsSender) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();

        let stats = [
            (
                ClientStatsType::Packets,
                Stats::Packets(packet_statistics::PacketStatisticsControl::default()),
            ),
            (
                ClientStatsType::Gateway,
                Stats::GatewayConn(gateway_conn_statistics::GatewayStatsControl::default()),
            ),
            (
                ClientStatsType::NymApi,
                Stats::NymApi(nym_api_statistics::NymApiStatsControl::default()),
            ),
        ];

        (
            StatisticsControl {
                stats,
                stats_rx,
                reporting_address,
                report_tx,
                stats_report: Default::default(),
            },
            ClientStatsSender::new(stats_tx),
        )
    }

    async fn report_stats(&mut self) {
        let mut metrics_report: HashMap<&str, String> = HashMap::new();
        for (_, stats) in self.stats.as_mut() {
            match stats.marshall() {
                Ok(metrics) => {
                    log::trace!(" {:?}: {:?}", stats.type_identity(), metrics);
                    metrics_report.insert(stats.type_identity().as_str(), metrics);
                }
                Err(err) => log::error!("{:?}: marshall metrics: {:?}", stats.type_identity(), err),
            }
            stats.periodic_reset();
        }

        let metrics = match serde_json::to_string(&metrics_report) {
            Ok(r) => r,
            Err(e) => {
                log::error!("failed to serialize metrics to json: {e:?}");
                return;
            }
        };

        if let Ok(mut report_bytes) = self.stats_report.marshall() {
            report_bytes.push_str(&metrics);
            let report_message = InputMessage::new_regular(
                self.reporting_address,
                report_bytes.as_bytes().to_vec(),
                TransmissionLane::General,
                None,
            );
            if let Err(err) = self.report_tx.send(report_message).await {
                log::error!("Failed to report client stats: {:?}", err);
            } else {
                self.stats_report = Default::default();
            }
        } else {
            log::error!("Failed to serialize stats report. This should never happen");
        }
    }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started StatisticsControl with graceful shutdown support");

        let report_interval = Duration::from_secs(STATS_REPORT_INTERVAL_SECS);
        let mut report_interval = tokio::time::interval(report_interval);
        let snapshot_interval = Duration::from_millis(SNAPSHOT_INTERVAL_MS);
        let mut snapshot_interval = tokio::time::interval(snapshot_interval);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                        Some(stats_event) => self.handle_event(stats_event),
                        None => {
                            log::trace!("StatisticsControl: shutting down due to closed stats channel");
                            break;
                        }
                },
                _ = snapshot_interval.tick() => {
                    for (_, stats) in self.stats.as_mut() {
                        stats.snapshot();
                    }
                }
                _ = report_interval.tick() => {
                    self.report_stats().await;
                }
                _ = shutdown.recv_with_delay() => {
                    log::trace!("StatisticsControl: Received shutdown");
                    break;
                },
            }
        }
        log::debug!("StatisticsControl: Exiting");
    }

    fn handle_event(&mut self, stats_event: nym_statistics_common::clients::ClientStatsEvents) {
        let event_type = stats_event.metrics_type();
        log::trace!("StatisticsControl: Received stats event ");
        let mut found = false;
        for (ident, stats) in self.stats.as_mut() {
            if ident == &event_type {
                stats.handle_event(stats_event);
                found = true;
                break;
            }
        }
        if !found {
            log::warn!(
                "received event for unregistered metrics type: {:?}",
                event_type
            );
        }
    }

    pub(crate) fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client).await;
        })
    }
}

#[cfg(test)]
mod test {
    // use std::sync::Arc;
    // use tokio::sync::Mutex;

    // use super::*;
    // use nym_statistics_common::clients::gateway_conn_statistics::GatewayStatsEvent;
    // use nym_statistics_common::clients::nym_api_statistics::NymApiStatsEvent;
    // use nym_statistics_common::clients::packet_statistics::PacketStatisticsEvent;

    // Disabled #[tokio::test]
    // async fn test_metrics_controller() {
    //     let _ = pretty_env_logger::try_init();
    //     let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    //     let (metrics_controller, metrics_sender) = StatisticsControl::new();
    //     let m = Arc::new(Mutex::new(metrics_controller));
    //     let m1 = Arc::clone(&m);
    //     tokio::spawn(async move {
    //         let mut mc = m1.lock().await;
    //         mc.run_with_shutdown(nym_task::TaskClient::dummy()).await;
    //         shutdown_tx.send(()).unwrap();
    //     });

    //     for _ in 0..10 {
    //         metrics_sender.report(StatsEvents::PacketStatistics(
    //             PacketStatisticsEvent::RealPacketSent(1),
    //         ));
    //         metrics_sender.report(StatsEvents::GatewayConn(GatewayStatsEvent::RealPacketSent(
    //             2,
    //         )));
    //         metrics_sender.report(StatsEvents::NymApi(NymApiStatsEvent::RealPacketSent(3)));
    //         tokio::time::sleep(Duration::from_millis(500)).await;
    //     }

    //     drop(metrics_sender);
    //     shutdown_rx.await.unwrap();
    // }
}
