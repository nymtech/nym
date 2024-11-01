// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! # Statistics collection and reporting.
//!
//! Modular metrics collection and reporting system. submodules can be added to collect different types of metrics.
//! On creation the Statistics controller will start a task that will listen for incoming stats events and
//! multiplex them out to the appropriate metrics module based on type.
//!
//! Adding A new module you need to write a new module that implements the `StatsObj` trait and add it to
//! the `stats` hashmap in the `StatisticsController` struct during it's initialization in the `new` function in
//! this file.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use std::{collections::HashMap, time::Duration};

use nym_sphinx::addressing::Recipient;
use nym_statistics_common::{
    clients::{ClientStatsEvent, ClientStatsReceiver, ClientStatsReporter},
    report::ClientStatsReport,
};
use nym_task::connections::TransmissionLane;

use crate::{
    client::inbound_messages::{InputMessage, InputMessageSender},
    spawn_future,
};

pub(crate) mod gateway_conn_statistics;
pub(crate) mod nym_api_statistics;
pub(crate) mod packet_statistics;

// Time interval between reporting packet statistics
const STATS_REPORT_INTERVAL_SECS: u64 = 300;
// Interval for taking snapshots of the packet statistics
const SNAPSHOT_INTERVAL_MS: u64 = 500;

#[derive(PartialEq, Eq, Hash, Debug)]
pub(crate) enum StatsType {
    Packets,
    Gateway,
    NymApi,
}

pub(crate) enum StatsEvents {
    PacketStatistics(packet_statistics::PacketStatisticsEvent),
    GatewayConn(gateway_conn_statistics::GatewayStatsEvent),
    NymApi(nym_api_statistics::NymApiStatsEvent),
}

impl StatsEvents {
    pub(crate) fn metrics_type(&self) -> StatsType {
        match self {
            StatsEvents::PacketStatistics(_) => StatsType::Packets,
            StatsEvents::GatewayConn(_) => StatsType::Gateway,
            StatsEvents::NymApi(_) => StatsType::NymApi,
        }
    }
}

type StatisticsReceiver = tokio::sync::mpsc::UnboundedReceiver<StatsEvents>;

#[derive(Clone)]
pub(crate) struct ClientStatisticsSender {
    stats_tx: tokio::sync::mpsc::UnboundedSender<StatsEvents>,
}

impl ClientStatisticsSender {
    pub(crate) fn new(stats_tx: tokio::sync::mpsc::UnboundedSender<StatsEvents>) -> Self {
        ClientStatisticsSender { stats_tx }
    }

    pub(crate) fn report(&self, event: StatsEvents) {
        if let Err(err) = self.stats_tx.send(event) {
            log::error!("Failed to send stats event: {:?}", err);
        }
    }
}

pub(crate) trait StatsObj: StatisticsReporter + Send {
    fn new() -> Self
    where
        Self: Sized;

    fn type_identity(&self) -> StatsType;

    /// Handle an incoming stats event
    fn handle_event(&mut self, event: StatsEvents);

    /// snapshot the current state of the metrics if the module wishes to use it
    fn snapshot(&mut self);

    /// Reset the metrics to their initial state.
    ///
    /// Used to periodically reset the metrics in accordance with periodic reporting strategy
    fn periodic_reset(&mut self);
}

/// This trait represents objects that can be reported by the metrics controller and
/// provides the function by which they will be called to report their metrics.
pub(crate) trait StatisticsReporter {
    /// Marshall the metrics into a string and write them to the provided formatter.
    fn marshall(&self) -> std::io::Result<String>;
}

/// Launches and manages metrics collection and reporting.
///
/// This is designed to be generic to allow for multiple types of metrics to be collected and
/// reported.
pub(crate) struct StatisticsControl {
    /// Keep store the different types of metrics collectors
    stats: HashMap<StatsType, Box<dyn StatsObj>>,

    /// Incoming packet stats events from other tasks
    stats_rx: StatisticsReceiver,

    /// Channel to send stats report through the mixnet
    report_tx: InputMessageSender,

    /// Service-provider address to send stats reports
    reporting_address: Recipient,

    /// ???
    stats_report: ClientStatsReport,
}

impl StatisticsControl {
    pub(crate) fn new(
        reporting_address: Recipient,
        report_tx: InputMessageSender,
    ) -> (Self, ClientStatisticsSender) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut stats: HashMap<StatsType, Box<dyn StatsObj>> = HashMap::new();
        stats.insert(
            StatsType::Packets,
            Box::new(packet_statistics::PacketStatisticsControl::new()),
        );

        stats.insert(
            StatsType::Gateway,
            Box::new(gateway_conn_statistics::GatewayStatsControl::new()),
        );
        stats.insert(
            StatsType::NymApi,
            Box::new(nym_api_statistics::NymApiStatsControl::new()),
        );

        (
            StatisticsControl {
                stats,
                stats_rx,
                reporting_address,
                report_tx,
                stats_report: Default::default(),
            },
            ClientStatisticsSender::new(stats_tx),
        )
    }

    async fn report_stats(&mut self) {
        if let Ok(report_bytes) = self.stats_report.clone().try_into() {
            let report_message = InputMessage::new_regular(
                self.reporting_address,
                report_bytes,
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

    pub(crate) fn report_all(&mut self) {
        for stats in self.stats.values_mut() {
            match stats.marshall() {
                Ok(metrics) => log::info!(" {:?}: {:?}", stats.type_identity(), metrics),
                Err(err) => log::error!("{:?}: marshall metrics: {:?}", stats.type_identity(), err),
            }
            stats.periodic_reset();
        }
    }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started StatisticsController with graceful shutdown support");

        let report_interval = Duration::from_secs(STATS_REPORT_INTERVAL_SECS);
        let mut report_interval = tokio::time::interval(report_interval);
        let snapshot_interval = Duration::from_millis(SNAPSHOT_INTERVAL_MS);
        let mut snapshot_interval = tokio::time::interval(snapshot_interval);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                    Some(stats_event) => {
                        log::trace!("StatisticsController: Received stats event");
                        match self.stats.get_mut(&stats_event.metrics_type()) {
                            Some(stats) => stats.handle_event(stats_event),
                            None => log::warn!("received event for unregistered metrics type: {:?}", stats_event.metrics_type()),
                        }
                    },
                    None => {
                        log::trace!("StatisticsController: stopping since stats channel was closed");
                        break;
                    }
                },
                _ = snapshot_interval.tick() => {
                    for stats in self.stats.values_mut() {
                        stats.snapshot();
                    }
                }
                _ = report_interval.tick() => {
                    // self.report_all();
                    self.report_stats().await;
                }
                _ = shutdown.recv_with_delay() => {
                    log::trace!("StatisticsController: Received shutdown");
                    break;
                },
            }
        }
        log::debug!("StatisticsController: Exiting");
    }

    pub(crate) fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client).await;
        })
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use super::*;
    use crate::client::statistics::gateway_conn_statistics::GatewayStatsEvent;
    use crate::client::statistics::nym_api_statistics::NymApiStatsEvent;
    use crate::client::statistics::packet_statistics::PacketStatisticsEvent;

    #[tokio::test]
    async fn test_metrics_controller() {
        let _ = pretty_env_logger::try_init();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let (metrics_controller, metrics_sender) = StatisticsControl::new();
        let m = Arc::new(Mutex::new(metrics_controller));
        let m1 = Arc::clone(&m);
        tokio::spawn(async move {
            let mut mc = m1.lock().await;
            mc.run_with_shutdown(nym_task::TaskClient::dummy()).await;
            shutdown_tx.send(()).unwrap();
        });

        for _ in 0..10 {
            metrics_sender.report(StatsEvents::PacketStatistics(
                PacketStatisticsEvent::RealPacketSent(1),
            ));
            metrics_sender.report(StatsEvents::GatewayConn(GatewayStatsEvent::RealPacketSent(
                2,
            )));
            metrics_sender.report(StatsEvents::NymApi(NymApiStatsEvent::RealPacketSent(3)));
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        drop(metrics_sender);
        shutdown_rx.await.unwrap();
    }
}
