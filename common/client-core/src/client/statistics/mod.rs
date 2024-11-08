// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use std::time::Duration;

use nym_sphinx::addressing::Recipient;
use nym_statistics_common::{
    clients::{ClientStatsEvent, ClientStatsReceiver, ClientStatsReporter},
    report::ClientStatsReport,
};
use nym_task::connections::TransmissionLane;

use crate::spawn_future;

use super::inbound_messages::{InputMessage, InputMessageSender};

// Time interval between reporting statistics
const STATS_REPORT_INTERVAL_SECS: u64 = 300;

pub(crate) struct StatisticsControl {
    // Incoming stats events from other tasks
    stats_rx: ClientStatsReceiver,

    //service-provider address to send stats reports
    reporting_address: Recipient,

    //channel to send stats report through the mixnet
    report_tx: InputMessageSender,

    stats_report: ClientStatsReport,
}

impl StatisticsControl {
    pub(crate) fn new(
        reporting_address: Recipient,
        report_tx: InputMessageSender,
    ) -> (Self, ClientStatsReporter) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            StatisticsControl {
                stats_rx,
                reporting_address,
                report_tx,
                stats_report: Default::default(),
            },
            ClientStatsReporter::new(stats_tx),
        )
    }

    fn handle_event(&mut self, _event: ClientStatsEvent) {
        unimplemented!("No supported events for now");
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

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started StatisticsControl with graceful shutdown support");

        let report_interval = Duration::from_secs(STATS_REPORT_INTERVAL_SECS);
        let mut report_interval = tokio::time::interval(report_interval);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                    Some(stats_event) => {
                        log::trace!("StatisticsControl: Received stats event");
                        self.handle_event(stats_event);
                    },
                    None => {
                        log::trace!("StatisticsControl: stopping since stats channel was closed");
                        break;
                    }
                },
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
}
