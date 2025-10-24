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

use crate::client::inbound_messages::{InputMessage, InputMessageSender};
use futures::StreamExt;
use nym_client_core_config_types::StatsReporting;
use nym_sphinx::addressing::Recipient;
use nym_statistics_common::clients::{
    ClientStatsController, ClientStatsReceiver, ClientStatsSender,
};
use nym_task::{connections::TransmissionLane, ShutdownToken, ShutdownTracker};
use std::time::Duration;

/// Time interval between reporting statistics locally (logging/shutdown_token)
const LOCAL_REPORT_INTERVAL: Duration = Duration::from_secs(2);
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

    /// Config for stats reporting (enabled, address, interval)
    reporting_config: StatsReporting,
}

impl StatisticsControl {
    pub(crate) fn create(
        reporting_config: StatsReporting,
        client_type: String,
        client_stats_id: String,
        report_tx: InputMessageSender,
        shutdown_token: ShutdownToken,
    ) -> (Self, ClientStatsSender) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();

        let stats = ClientStatsController::new(client_stats_id, client_type);

        (
            StatisticsControl {
                stats,
                stats_rx,
                report_tx,
                reporting_config,
            },
            ClientStatsSender::new(Some(stats_tx), shutdown_token),
        )
    }

    async fn report_stats(&mut self, recipient: Recipient) {
        let stats_report = self.stats.build_report();

        let report_message = InputMessage::new_regular(
            recipient,
            stats_report.into(),
            TransmissionLane::General,
            None,
            #[cfg(feature = "otel")]
            None,
        );
        if let Err(err) = self.report_tx.send(report_message).await {
            tracing::error!("Failed to report client stats: {err:?}");
        } else {
            self.stats.reset();
        }
    }

    // manually control the shutdown mechanism as we don't want to get interrupted mid-snapshot
    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        tracing::debug!("Started StatisticsControl with graceful shutdown support");

        #[cfg(not(target_arch = "wasm32"))]
        let mut stats_report_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(self.reporting_config.reporting_interval),
        );

        #[cfg(not(target_arch = "wasm32"))]
        let mut local_report_interval = tokio_stream::wrappers::IntervalStream::new(
            tokio::time::interval(LOCAL_REPORT_INTERVAL),
        );

        #[cfg(not(target_arch = "wasm32"))]
        let mut snapshot_interval =
            tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(SNAPSHOT_INTERVAL));

        #[cfg(target_arch = "wasm32")]
        let mut stats_report_interval = gloo_timers::future::IntervalStream::new(
            self.reporting_config.reporting_interval.as_millis() as u32,
        );

        #[cfg(target_arch = "wasm32")]
        let mut local_report_interval =
            gloo_timers::future::IntervalStream::new(LOCAL_REPORT_INTERVAL.as_millis() as u32);

        #[cfg(target_arch = "wasm32")]
        let mut snapshot_interval =
            gloo_timers::future::IntervalStream::new(SNAPSHOT_INTERVAL.as_millis() as u32);

        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    tracing::trace!("StatisticsControl: Received shutdown");
                    break;
                },
                stats_event = self.stats_rx.recv() => match stats_event {
                        Some(stats_event) => self.stats.handle_event(stats_event),
                        None => {
                            tracing::trace!("StatisticsControl: shutting down due to closed stats channel");
                            break;
                        }
                },
                _ = snapshot_interval.next() => {
                    self.stats.snapshot();
                }
                _ = stats_report_interval.next() => {
                    let Some(recipient) = self.reporting_config.provider_address else {
                        continue
                    };

                    if self.reporting_config.enabled {
                        self.report_stats(recipient).await;
                    }
                }

                _ = local_report_interval.next() => {
                    self.stats.local_report();
                }
            }
        }
        tracing::debug!("StatisticsControl: Exiting");
    }

    pub(crate) fn create_and_start(
        reporting_config: StatsReporting,
        client_type: String,
        client_stats_id: String,
        report_tx: InputMessageSender,
        shutdown_tracker: &ShutdownTracker,
    ) -> ClientStatsSender {
        let (mut controller, sender) = Self::create(
            reporting_config,
            client_type,
            client_stats_id,
            report_tx,
            shutdown_tracker.child_shutdown_token(),
        );
        let shutdown_token = shutdown_tracker.clone_shutdown_token();
        shutdown_tracker.try_spawn_named(
            async move {
                controller.run(shutdown_token).await;
            },
            "StatisticsControl",
        );
        sender
    }
}
