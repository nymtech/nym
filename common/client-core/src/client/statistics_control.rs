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

use std::time::Duration;

use nym_sphinx::addressing::Recipient;
use nym_statistics_common::{
    clients::{ClientStatsController, ClientStatsReceiver, ClientStatsSender},
    StatsReportingConfig,
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

/// Launches and manages metrics collection and reporting.
///
/// This is designed to be generic to allow for multiple types of metrics to be collected and
/// reported.
pub(crate) struct StatisticsControl {
    /// Keep store the different types of metrics collectors
    stats: ClientStatsController,

    /// Incoming packet stats events from other tasks
    stats_rx: ClientStatsReceiver,

    /// Channel to send stats report through the mixnet
    report_tx: InputMessageSender,

    /// Service-provider address to send stats reports
    reporting_address: Recipient,
}

impl StatisticsControl {
    pub(crate) fn create(
        reporting_config: StatsReportingConfig,
        client_stats_id: String,
        report_tx: InputMessageSender,
    ) -> (Self, ClientStatsSender) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();
        let client_type = reporting_config.reporting_type;
        let reporting_address = reporting_config.reporting_address;

        let stats = ClientStatsController::new(client_stats_id, client_type);

        (
            StatisticsControl {
                stats,
                stats_rx,
                reporting_address,
                report_tx,
            },
            ClientStatsSender::new(Some(stats_tx)),
        )
    }

    async fn report_stats(&mut self) {
        let stats_report = self.stats.build_report();

        if let Ok(report_bytes) = stats_report.try_into() {
            let report_message = InputMessage::new_regular(
                self.reporting_address,
                report_bytes,
                TransmissionLane::General,
                None,
            );
            if let Err(err) = self.report_tx.send(report_message).await {
                log::error!("Failed to report client stats: {:?}", err);
            } else {
                self.stats.reset();
            }
        } else {
            log::error!("Failed to serialize stats report. This should never happen");
        }
    }

    async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started StatisticsControl with graceful shutdown support");

        let report_interval = Duration::from_secs(STATS_REPORT_INTERVAL_SECS);
        let mut report_interval = tokio::time::interval(report_interval);
        let snapshot_interval = Duration::from_millis(SNAPSHOT_INTERVAL_MS);
        let mut snapshot_interval = tokio::time::interval(snapshot_interval);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                        Some(stats_event) => self.stats.handle_event(stats_event),
                        None => {
                            log::trace!("StatisticsControl: shutting down due to closed stats channel");
                            break;
                        }
                },
                _ = snapshot_interval.tick() => {
                    self.stats.snapshot();
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

    pub(crate) fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client).await;
        })
    }

    pub(crate) fn create_and_start_with_shutdown(
        reporting_config: Option<StatsReportingConfig>,
        client_stats_id: String,
        report_tx: InputMessageSender,
        task_client: nym_task::TaskClient,
    ) -> ClientStatsSender {
        match reporting_config {
            None => ClientStatsSender::new(None),
            Some(cfg) => {
                let (controller, sender) = Self::create(cfg, client_stats_id, report_tx);
                controller.start_with_shutdown(task_client);
                sender
            }
        }
    }
}
