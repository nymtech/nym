// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

/// Time interval between reporting statistics to the given provider
const STATS_REPORT_INTERVAL_SECS: u64 = 300;
/// Time interval between reporting statistics to the task client
const TASK_CLIENT_REPORT_INTERVAL: Duration = Duration::from_secs(2);
/// Interval for taking snapshots of the statistics
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(500);

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
    reporting_address: Option<Recipient>,
}

impl StatisticsControl {
    pub(crate) fn create(
        reporting_config: Option<StatsReportingConfig>,
        client_stats_id: String,
        report_tx: InputMessageSender,
    ) -> (Self, ClientStatsSender) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();
        let (reporting_address, client_type) = match reporting_config {
            Some(cfg) => (Some(cfg.reporting_address), cfg.reporting_type),
            None => (None, "".into()),
        };

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

    async fn report_stats(&mut self, recipient: Recipient) {
        let stats_report = self.stats.build_report();

        let report_message = InputMessage::new_regular(
            recipient,
            stats_report.into(),
            TransmissionLane::General,
            None,
        );
        if let Err(err) = self.report_tx.send(report_message).await {
            log::error!("Failed to report client stats: {:?}", err);
        } else {
            self.stats.reset();
        }
    }

    async fn run_with_shutdown(&mut self, mut task_client: nym_task::TaskClient) {
        log::debug!("Started StatisticsControl with graceful shutdown support");

        let stats_report_interval = Duration::from_secs(STATS_REPORT_INTERVAL_SECS);
        let mut stats_report_interval = tokio::time::interval(stats_report_interval);
        let mut task_client_report_interval = tokio::time::interval(TASK_CLIENT_REPORT_INTERVAL);
        let mut snapshot_interval = tokio::time::interval(SNAPSHOT_INTERVAL);

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
                _ = stats_report_interval.tick(), if self.reporting_address.is_some() => {
                    // SAFTEY : this branch executes only if reporting is not none, so unwrapp is fine
                    #[allow(clippy::unwrap_used)]
                    self.report_stats(self.reporting_address.unwrap()).await;
                }

                _ = task_client_report_interval.tick() => {
                    self.stats.task_client_report(&mut task_client);
                }
                _ = task_client.recv_with_delay() => {
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
        let (controller, sender) = Self::create(reporting_config, client_stats_id, report_tx);
        controller.start_with_shutdown(task_client);
        sender
    }
}
